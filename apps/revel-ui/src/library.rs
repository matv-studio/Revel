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
use crate::panels::Search;
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
        // Ordre d'affichage : dossiers renommés suivis.
        order: s
            .order
            .iter()
            .map(|k| match k.strip_prefix("folder:") {
                Some(f) => format!("folder:{}", moved(f)),
                None => k.clone(),
            })
            .collect(),
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
        order: s.order.clone(),
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
    name_input(ui, initial, "library-folder-name", commit, cancel)
}

/// Nom d'un dossier saisi sur place (pages : renommage).
pub(crate) fn folder_name_input(
    ui: Ui,
    initial: String,
    commit: impl Fn(String) -> bool + Copy + 'static,
    cancel: impl Fn() + Copy + 'static,
) -> impl IntoView {
    name_input(ui, initial, "library-folder-name", commit, cancel)
}

/// Ligne de saisie d'un nom (nouveau dossier, nouvelle page) : icône et champ.
pub(crate) fn input_row(
    ui: Ui,
    icon: AnyView,
    depth: usize,
    key: &'static str,
    commit: impl Fn(String) -> bool + Copy + 'static,
    cancel: impl Fn() + Copy + 'static,
) -> AnyView {
    view! {
        <div class="list-row list-folder is-editing" style=format!("--depth:{depth}")>
            <span class="list-chevron"></span>
            <span class="list-icon">{icon}</span>
            {name_input(ui, String::new(), key, commit, cancel)}
        </div>
    }
    .into_any()
}

/// Champ de nom saisi sur place (`key` : libellé et texte d'exemple).
fn name_input(
    ui: Ui,
    initial: String,
    key: &'static str,
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
            aria-label=ui.t(key) placeholder=ui.t(key)
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
    folder_row_with(sh, path, depth, count, chevron, open, None)
}

/// Ligne de dossier ; `toggle` : la flèche déplie le dossier sur place (le clic sur la ligne
/// garde son action, entrer dans le dossier).
fn folder_row_with(
    sh: Shelves,
    path: String,
    depth: usize,
    count: usize,
    chevron: Option<Signal<bool>>,
    open: impl Fn() + Copy + Send + Sync + 'static,
    toggle: Option<Callback<()>>,
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
            data-drop-folder=path.get_value()
            title=path.get_value()
            class:is-drop=move || sh.drop_on.get().as_deref() == Some(path.get_value().as_str())
            on:click=move |_| if !renaming() { open() }
            on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" && !renaming() { open() }
            on:contextmenu=move |e: leptos::ev::MouseEvent| sh.folder_menu(&e, path.get_value(), open)
            on:dragover=move |e: leptos::ev::DragEvent| drop(e, false)
            on:dragleave=move |_| sh.drop_on.set(None)
            on:drop=move |e: leptos::ev::DragEvent| drop(e, true)>
            {match toggle {
                Some(t) => view! {
                    <button class="list-chevron list-chevron--toggle" tabindex="-1"
                        title=sh.ui.t("media-folder-expand")
                        aria-expanded=move || chevron.is_some_and(|c| c.get()).to_string()
                        on:click=move |e: leptos::ev::MouseEvent| { e.stop_propagation(); t.run(()) }
                        on:dblclick=move |e: leptos::ev::MouseEvent| e.stop_propagation()>
                        {move || if chevron.is_some_and(|c| c.get()) { icons::chevron_down() } else { icons::chevron_right() }}
                    </button>
                }.into_any(),
                None => view! {
                    <span class="list-chevron">{chevron.map(|c| move || if c.get() { icons::chevron_down() } else { icons::chevron_right() })}</span>
                }.into_any(),
            }}
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
    // Dossiers dépliés sur place (flèche), gardés d'une session à l'autre.
    let expanded = RwSignal::new(
        crate::pref_get("media.open")
            .map(|v| {
                v.split('\n')
                    .filter(|x| !x.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    );
    let toggle_folder = move |p: String| {
        expanded.update(|e| {
            if let Some(i) = e.iter().position(|x| *x == p) {
                e.remove(i);
            } else {
                e.push(p);
            }
        });
        crate::pref_set("media.open", &expanded.with_untracked(|e| e.join("\n")));
    };
    // Liste relue à l'ouverture du panneau et après une modification de fichiers hors de Revel.
    let path = ed.path.get_value();
    let refresh = RwSignal::new(0u32);
    // Média ouvert dans le volet de détails ; dépôt de fichiers du système en cours.
    let details = RwSignal::new(None::<String>);
    let dropping = RwSignal::new(false);
    Effect::new(move |_| {
        ed.load.track();
        refresh.track();
        ui.media_rev.track();
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
    // Médias d'un dossier (ou trouvés par la recherche), au filtre de type et à l'ordre choisis.
    let select = move |dir: Option<&str>, ql: &str| {
        let k = kind.get();
        let mut v: Vec<MediaFile> = sh.shelf.with(|s| {
            files.with(|fs| {
                fs.iter()
                    .filter(|f| k.is_none_or(|k| f.kind == k))
                    .filter(|f| match dir {
                        Some(d) => folder_of(s, &f.url) == d,
                        None => file_name(&f.url).to_lowercase().contains(ql),
                    })
                    .cloned()
                    .collect()
            })
        });
        sort_media(&mut v, sort.get());
        v
    };
    let shown = Memo::new(move |_| {
        let ql = q.get().trim().to_lowercase();
        let cur = folder.get();
        if ql.is_empty() {
            select(Some(&cur), "")
        } else {
            select(None, &ql)
        }
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
    // Glisser une vignette : sur un dossier (ou le fil d'Ariane) pour la ranger, sur la page pour
    // l'y placer ; un clic ouvre ses détails.
    let drag = crate::insert::CanvasDrag::new(ui, ed);
    let folder_under = |cx: f64, cy: f64| -> Option<String> {
        document()
            .element_from_point(cx as f32, cy as f32)?
            .closest("[data-drop-folder]")
            .ok()??
            .get_attribute("data-drop-folder")
    };
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
        let kind = f.kind;
        let menu = move |e: leptos::ev::MouseEvent| {
            use crate::ctxmenu::{CtxItem, open};
            let (u1, u2, u3) = (url.get_value(), url.get_value(), url.get_value());
            let mut items = vec![
                CtxItem::new(ui.now("media-details", &[]), move || {
                    details.set(Some(u1.clone()))
                })
                .icon(icons::info),
                CtxItem::new(ui.now("media-insert", &[]), move || {
                    insert_media(ui, ed, &u2, kind)
                })
                .icon(icons::plus),
                CtxItem::new(ui.now("media-copy-url", &[]), move || api::copy_text(&u3))
                    .icon(icons::link),
            ];
            items.extend(sh.move_items(&url.get_value()));
            open(ui, &e, items);
        };
        let item = move || media_drag_item(ed, &url.get_value(), kind);
        view! {
            <figure class="asset" title=tip data-url=f.url.clone()
                class:is-dragging=move || sh.dragging.get().as_deref() == Some(url.get_value().as_str())
                on:contextmenu=menu
                on:pointerdown=move |e: leptos::ev::PointerEvent| drag.down(&e)
                on:pointermove=move |e: leptos::ev::PointerEvent| {
                    if drag.moved(&e, item()) {
                        if sh.dragging.get_untracked().is_none() {
                            sh.dragging.set(Some(url.get_value()));
                        }
                        let f = folder_under(f64::from(e.client_x()), f64::from(e.client_y()));
                        if sh.drop_on.get_untracked() != f {
                            sh.drop_on.set(f);
                        }
                    }
                }
                on:pointerup=move |e: leptos::ev::PointerEvent| {
                    let over = folder_under(f64::from(e.client_x()), f64::from(e.client_y()));
                    sh.dragging.set(None);
                    sh.drop_on.set(None);
                    match drag.finish(Some(&e)) {
                        None => details.set(Some(url.get_value())),
                        Some((it, Some((parent, index)))) => crate::insert::insert_item(ui, ed, &it, parent, index),
                        Some((_, None)) => if let Some(f) = over { sh.put(&url.get_value(), &f) },
                    }
                }
                on:pointercancel=move |_| { drag.finish(None); sh.dragging.set(None); sh.drop_on.set(None); }>
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
    // Import : sélecteur ou fichiers glissés depuis le système, dans le dossier affiché.
    let import = move |fs: Vec<web_sys::File>| {
        if fs.is_empty() {
            return;
        }
        let path = ed.path.get_value();
        let target = folder.get_untracked();
        spawn_local(async move {
            let mut urls = Vec::new();
            for f in fs {
                let Ok(bytes) = crate::files::read(&f).await else {
                    continue;
                };
                match api::edit_import_media(&path, &f.name(), &bytes).await {
                    Ok(m) => urls.push(m.url),
                    Err(e) => ui.fail(e),
                }
            }
            if !urls.is_empty() && !target.is_empty() {
                let s = sh.shelf.with_untracked(|s| {
                    urls.iter().fold(s.clone(), |acc, u| put(&acc, u, &target))
                });
                sh.set(s);
            }
            if !urls.is_empty() {
                refresh.update(|n| *n += 1);
            }
        });
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
                <button class="media-crumb" class:is-last=last data-drop-folder=t2.clone()
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
        <div class="panel-list media" class:is-dropping=move || dropping.get()
            on:dragover=move |e: leptos::ev::DragEvent| {
                if crate::files::has_files(&e) {
                    e.prevent_default();
                    dropping.set(true);
                }
            }
            on:dragleave=move |e: leptos::ev::DragEvent| {
                // Sortie du panneau (pas d'un de ses enfants).
                let inside = e.related_target()
                    .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Node>(t).ok())
                    .zip(e.current_target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Node>(t).ok()))
                    .is_some_and(|(r, c)| c.contains(Some(&r)));
                if !inside { dropping.set(false) }
            }
            on:drop=move |e: leptos::ev::DragEvent| {
                if crate::files::has_files(&e) {
                    e.prevent_default();
                    dropping.set(false);
                    import(crate::files::list(e.data_transfer().and_then(|dt| dt.files())));
                }
            }>
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
                        subs.into_iter().map(|p| media_folder(MediaTree {
                            sh, counts, expanded, folder, thumb,
                            toggle: Callback::new(toggle_folder),
                            files_in: Callback::new(move |d: String| select(Some(&d), "")),
                        }, p, 0, tile)).collect_view()
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
                <input class="range range--plain thumb-size" type="range" min=THUMB_MIN max=THUMB_MAX step="4"
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
                <button class="icon-btn icon-btn--small" title=ui.t("assets-import") aria-label=ui.t("assets-import")
                    on:click=move |_| crate::files::pick(crate::files::MEDIA_ACCEPT, true, import)>{icons::upload()}</button>
            </footer>
            {move || details.get().map(|u| view! {
                <MediaDetails url=u files=files on_close=Callback::new(move |_| details.set(None))
                    on_replaced=Callback::new(move |_| { refresh.update(|n| *n += 1); ed.load.update(|n| *n += 1); })/>
            })}
        </div>
    }
}

/// Range des médias dans l'ordre choisi.
fn sort_media(v: &mut [MediaFile], sort: MediaSort) {
    let name = |f: &MediaFile| file_name(&f.url).to_lowercase();
    match sort {
        MediaSort::Recent => v.sort_by(|a, b| b.modified.cmp(&a.modified).then(name(a).cmp(&name(b)))),
        MediaSort::Oldest => v.sort_by(|a, b| a.modified.cmp(&b.modified).then(name(a).cmp(&name(b)))),
        MediaSort::Name => v.sort_by_key(name),
        MediaSort::NameDesc => v.sort_by_key(|f| std::cmp::Reverse(name(f))),
        MediaSort::Kind => v.sort_by(|a, b| a.kind.cmp(&b.kind).then(name(a).cmp(&name(b)))),
        MediaSort::Size => v.sort_by_key(|f| std::cmp::Reverse(f.size)),
    }
}

/// État partagé de l'arbre des dossiers de médias.
#[derive(Clone, Copy)]
struct MediaTree {
    sh: Shelves,
    counts: Memo<BTreeMap<String, usize>>,
    /// Dossiers dépliés sur place.
    expanded: RwSignal<Vec<String>>,
    /// Dossier affiché (clic sur une ligne : on y entre).
    folder: RwSignal<String>,
    thumb: RwSignal<u32>,
    toggle: Callback<String>,
    files_in: Callback<String, Vec<MediaFile>>,
}

/// Dossier de médias : sa ligne, puis, s'il est déplié, ses sous-dossiers et ses médias en
/// retrait (sans y entrer).
fn media_folder<V: IntoView + 'static>(
    t: MediaTree,
    path: String,
    depth: usize,
    tile: impl Fn(MediaFile) -> V + Copy + Send + Sync + 'static,
) -> AnyView {
    let key = StoredValue::new(path.clone());
    let n = t.counts.with(|c| c.get(&path).copied().unwrap_or(0));
    let has_subs = t.sh.shelf.with(|s| !children(s, &path).is_empty());
    let is_open = Signal::derive(move || key.with_value(|k| t.expanded.with(|e| e.contains(k))));
    let toggle = (n > 0 || has_subs).then(|| Callback::new(move |_| t.toggle.run(key.get_value())));
    view! {
        {folder_row_with(t.sh, path, depth, n, toggle.is_some().then_some(is_open),
            move || t.folder.set(key.get_value()), toggle)}
        {move || is_open.get().then(|| {
            let p = key.get_value();
            let subs = t.sh.shelf.with(|s| children(s, &p));
            let items = t.files_in.run(p);
            view! {
                {subs.into_iter().map(|s| media_folder(t, s, depth + 1, tile)).collect_view()}
                {(!items.is_empty()).then(|| view! {
                    <div class="asset-grid is-nested" class:is-small=move || t.thumb.get() < 60
                        style=move || format!("--thumb:{}px;--depth:{}", t.thumb.get(), depth + 1)>
                        {items.into_iter().map(tile).collect_view()}
                    </div>
                })}
            }
        })}
    }
    .into_any()
}

/// Élément glissé pour un média : image (texte alternatif par défaut) ou lien vers le fichier.
fn media_drag_item(ed: Ed, url: &str, kind: MediaKind) -> crate::editor::DragItem {
    let name = file_name(url);
    let q = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    let node = if kind == MediaKind::Image {
        let alt = ed
            .doc
            .with_untracked(|d| d.as_ref().and_then(|d| d.library.alt.get(url).cloned()))
            .unwrap_or_default();
        revel_model::Node::Image(revel_model::ImageNode {
            src: q(url),
            alt: Some(q(&alt)),
            ..Default::default()
        })
    } else {
        revel_model::Node::Element {
            id: None,
            tag: "a".into(),
            attrs: vec![revel_model::Attr {
                name: "href".into(),
                value: Some(url.to_string()),
                when: None,
            }],
            children: vec![revel_model::Node::Text { text: name.clone() }],
        }
    };
    crate::editor::DragItem {
        preset: revel_edit::make::Preset::Image,
        text_key: "",
        label: name,
        tag: if kind == MediaKind::Image { "img" } else { "a" },
        block: false,
        node: Some(node),
    }
}

/// Place un média dans la page, à l'emplacement de la sélection.
fn insert_media(ui: Ui, ed: Ed, url: &str, kind: MediaKind) {
    let Some((parent, index)) = crate::insert::insertion_point(ed) else {
        ui.fail(revel_protocol::Failure::new("insert-no-place"));
        return;
    };
    crate::insert::insert_item(ui, ed, &media_drag_item(ed, url, kind), parent, index);
}

/// Détails d'un média, par-dessus la colonne du panneau : aperçu, adresse, taille, dimensions,
/// texte alternatif par défaut, remplacement du fichier (même adresse), placement dans la page.
#[component]
fn MediaDetails(
    url: String,
    files: RwSignal<Vec<MediaFile>>,
    on_close: Callback<()>,
    on_replaced: Callback<()>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let url = StoredValue::new(url);
    let file = Memo::new(move |_| {
        files.with(|fs| fs.iter().find(|f| url.with_value(|u| f.url == *u)).cloned())
    });
    let dims = RwSignal::new(None::<(u32, u32)>);
    let stamp = RwSignal::new(0u32);
    let root = NodeRef::<leptos::html::Div>::new();
    let rect = RwSignal::new(None::<(f64, f64, f64, f64)>);
    Effect::new(move |_| {
        if let Some(el) = root.get() {
            let host = el
                .parent_element()
                .and_then(|p| p.closest(".dock-col, .dock-float").ok().flatten());
            if let Some(h) = host {
                let b = h.get_bounding_client_rect();
                rect.set(Some((b.left(), b.top(), b.width(), b.height())));
            }
        }
    });
    let src = move || {
        let base = ed.base.get().unwrap_or_default();
        format!(
            "{}{}?v={}",
            base,
            url.get_value().trim_start_matches('/'),
            stamp.get()
        )
    };
    let alt = Signal::derive(move || {
        ed.doc.with(|d| {
            d.as_ref()
                .and_then(|d| url.with_value(|u| d.library.alt.get(u).cloned()))
                .unwrap_or_default()
        })
    });
    let replace = move || {
        let u = url.get_value();
        let ext = u
            .rsplit_once('.')
            .map(|(_, e)| format!(".{e}"))
            .unwrap_or_default();
        crate::files::pick(&ext, false, move |fs| {
            let Some(f) = fs.into_iter().next() else {
                return;
            };
            let path = ed.path.get_value();
            let u = u.clone();
            spawn_local(async move {
                let Ok(bytes) = crate::files::read(&f).await else {
                    return;
                };
                match api::edit_replace_media(&path, &u, &f.name(), &bytes).await {
                    Ok(_) => {
                        stamp.update(|n| *n += 1);
                        on_replaced.run(());
                    }
                    Err(e) => ui.fail(e),
                }
            });
        });
    };
    view! {
        <div class="page-settings-overlay media-details" node_ref=root
            style=move || rect.get().map(|(x, y, w, h)| format!("left:{x}px;top:{y}px;width:{w}px;height:{h}px")).unwrap_or_else(|| "visibility:hidden".into())>
            <div class="page-settings">
                <header class="page-settings-head">
                    <button class="icon-btn icon-btn--small" title=ui.t("action-back") aria-label=ui.t("action-back")
                        on:click=move |_| on_close.run(())>{icons::back()}</button>
                    <span class="page-settings-title">{move || file_name(&url.get_value())}</span>
                </header>
                {move || file.get().map(|f| {
                    let image = f.kind == MediaKind::Image;
                    view! {
                        <div class="page-settings-body">
                            <div class="media-preview" class:is-file=!image>
                                {if image {
                                    view! { <img src=src alt="" on:load=move |e| {
                                        if let Some(img) = e.target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlImageElement>(t).ok()) {
                                            dims.set(Some((img.natural_width(), img.natural_height())));
                                        }
                                    }/> }.into_any()
                                } else {
                                    view! { <span class="asset-icon">{kind_icon(f.kind)}</span> }.into_any()
                                }}
                            </div>
                            <dl class="media-facts">
                                <dt>{ui.t("media-url")}</dt>
                                <dd class="path">{f.url.clone()}
                                    <button class="icon-btn icon-btn--small" title=ui.t("media-copy-url") aria-label=ui.t("media-copy-url")
                                        on:click=move |_| api::copy_text(&url.get_value())>{icons::copy()}</button></dd>
                                <dt>{ui.t("media-size")}</dt>
                                <dd>{human_size(ui, f.size)}{move || dims.get().map(|(w, h)| format!(" · {w} × {h} px"))}</dd>
                            </dl>
                            {image.then(|| view! {
                                <crate::inspector::Field label=ui.t("media-alt") value=alt
                                    placeholder=ui.now("media-alt-help", &[])
                                    on_commit=move |v: String| ed.apply(ui, revel_edit::Command::SetMediaAlt { url: url.get_value(), alt: Some(v) }, false)/>
                            })}
                            <p class="page-settings-note muted">{ui.t("media-replace-help")}</p>
                        </div>
                        <footer class="panel-foot page-settings-foot">
                            <button class="btn btn--small" on:click=move |_| replace()>{icons::upload()}{ui.t("media-replace")}</button>
                            <span class="toolbar-spacer"></span>
                            <button class="btn btn--small" title=ui.t("media-insert") on:click=move |_| insert_media(ui, ed, &url.get_value(), f.kind)>{icons::plus()}{ui.t("media-insert-short")}</button>
                        </footer>
                    }
                })}
            </div>
        </div>
    }
}

// ------------------------------------------------------------------ choix d'un média

/// Demande de choix d'un média (fenêtre « Choisir une image »).
#[derive(Clone)]
pub struct MediaPick {
    /// Dossier du site.
    pub site: String,
    /// Images seulement.
    pub images: bool,
    /// Un seul type de média (animations Lottie).
    pub kind: Option<MediaKind>,
    /// Rangement des médias (dossiers), quand il est connu (éditeur).
    pub shelf: Option<Shelf>,
    /// Adresse choisie.
    pub on_pick: std::sync::Arc<dyn Fn(String) + Send + Sync>,
}

/// Ouvre le choix d'un média du site ouvert (`images` : images seulement ; `shelf` : dossiers
/// du rangement, dans l'éditeur).
pub fn pick_media(
    ui: Ui,
    images: bool,
    shelf: Option<Shelf>,
    on_pick: impl Fn(String) + Send + Sync + 'static,
) {
    let Some(site) = ui.site_path() else {
        return;
    };
    ui.pick_media.set(Some(MediaPick {
        site,
        images,
        kind: None,
        shelf,
        on_pick: std::sync::Arc::new(on_pick),
    }));
}

/// Ouvre le choix d'une animation Lottie du site ouvert.
pub fn pick_animation(
    ui: Ui,
    shelf: Option<Shelf>,
    on_pick: impl Fn(String) + Send + Sync + 'static,
) {
    let Some(site) = ui.site_path() else {
        return;
    };
    ui.pick_media.set(Some(MediaPick {
        site,
        images: false,
        kind: Some(MediaKind::Animation),
        shelf,
        on_pick: std::sync::Arc::new(on_pick),
    }));
}

/// Ouvre le choix d'une image du site ouvert.
pub fn pick_image(ui: Ui, shelf: Option<Shelf>, on_pick: impl Fn(String) + Send + Sync + 'static) {
    pick_media(ui, true, shelf, on_pick);
}

/// Rangement des médias du document ouvert dans l'éditeur.
pub fn media_shelf(ed: Ed) -> Option<Shelf> {
    ed.doc
        .with_untracked(|d| d.as_ref().map(|d| d.library.media.clone()))
}

/// Adresse d'une vignette (`base` : adresse du canevas).
pub fn thumb_src(base: &str, url: &str) -> String {
    if url.starts_with("http") || url.starts_with("data:") {
        url.to_string()
    } else {
        format!("{base}{}", url.trim_start_matches('/'))
    }
}

/// Adresse du canevas du site `site` (vignettes) : celle de l'éditeur, sinon un canevas ouvert
/// pour l'occasion.
pub fn canvas_base(ui: Ui, site: &str) -> Signal<Option<String>> {
    let site = site.to_string();
    let s2 = site.clone();
    if ui
        .canvas_base
        .with_untracked(|b| b.as_ref().is_none_or(|(p, _)| *p != site))
    {
        spawn_local(async move {
            if let Ok(b) = api::canvas_open(&s2, false).await
                && ui
                    .canvas_base
                    .with_untracked(|c| c.as_ref().is_none_or(|(p, _)| *p != s2))
            {
                ui.canvas_base.set(Some((s2, b)));
            }
        });
    }
    Signal::derive(move || {
        ui.canvas_base.with(|b| {
            b.as_ref()
                .filter(|(p, _)| *p == site)
                .map(|(_, b)| b.clone())
        })
    })
}

/// Fenêtre de choix d'un média : recherche, dossier, vignettes ; un clic choisit, « Importer »
/// ajoute des fichiers au site (un seul fichier importé est choisi aussitôt).
#[component]
pub fn MediaPicker() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let close = move || ui.pick_media.set(None);
    let _esc = window_event_listener(leptos::ev::keydown, move |e| {
        if e.key() == "Escape" && ui.pick_media.with_untracked(Option::is_some) {
            e.prevent_default();
            e.stop_propagation();
            close();
        }
    });
    move || {
        ui.pick_media.get().map(|req| {
            let req = StoredValue::new(req);
            let site = req.with_value(|r| r.site.clone());
            let images = req.with_value(|r| r.images);
            let only = req.with_value(|r| r.kind);
            let files = RwSignal::new(Vec::<MediaFile>::new());
            let loaded = RwSignal::new(false);
            let q = RwSignal::new(String::new());
            // `None` : tous les dossiers.
            let folder = RwSignal::new(None::<String>);
            let base = canvas_base(ui, &site);
            let path = StoredValue::new(site);
            Effect::new(move |_| {
                ui.media_rev.track();
                let p = path.get_value();
                spawn_local(async move {
                    match api::edit_media(&p).await {
                        Ok(m) => files.set(m),
                        Err(e) => ui.fail(e),
                    }
                    loaded.set(true);
                });
            });
            let choose = move |url: String| {
                let f = req.with_value(|r| r.on_pick.clone());
                close();
                f(url);
            };
            let shown = Memo::new(move |_| {
                let ql = q.get().trim().to_lowercase();
                let cur = folder.get();
                let mut v: Vec<MediaFile> = req.with_value(|r| {
                    files.with(|fs| {
                        fs.iter()
                            .filter(|f| !images || f.kind == MediaKind::Image)
                            .filter(|f| only.is_none_or(|k| f.kind == k))
                            .filter(|f| ql.is_empty() || file_name(&f.url).to_lowercase().contains(&ql))
                            .filter(|f| match (&cur, &r.shelf) {
                                (Some(c), Some(s)) => within(folder_of(s, &f.url), c),
                                _ => true,
                            })
                            .cloned()
                            .collect()
                    })
                });
                v.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.url.cmp(&b.url)));
                v
            });
            let folders = req.with_value(|r| r.shelf.as_ref().map(|s| {
                let mut f = s.folders.clone();
                f.sort();
                f
            }).unwrap_or_default());
            let import = move |fs: Vec<web_sys::File>| {
                if fs.is_empty() {
                    return;
                }
                let p = path.get_value();
                spawn_local(async move {
                    let mut urls = Vec::new();
                    for f in fs {
                        let Ok(bytes) = crate::files::read(&f).await else { continue };
                        match api::edit_import_media(&p, &f.name(), &bytes).await {
                            Ok(m) => urls.push(m.url),
                            Err(e) => ui.fail(e),
                        }
                    }
                    if urls.is_empty() {
                        return;
                    }
                    ui.media_rev.update(|n| *n += 1);
                    if urls.len() == 1 && ui.pick_media.with_untracked(Option::is_some) {
                        choose(urls.remove(0));
                    }
                });
            };
            let accept = if images {
                "image/*"
            } else if only == Some(MediaKind::Animation) {
                ".json,.lottie"
            } else {
                crate::files::MEDIA_ACCEPT
            };
            let tile = move |f: MediaFile| {
                let url = f.url.clone();
                let name = file_name(&f.url);
                let src = {
                    let u = f.url.clone();
                    move || base.get().map(|b| thumb_src(&b, &u))
                };
                let kind = f.kind;
                view! {
                    <button type="button" class="asset picker-tile" title=format!("{}\n{}", f.url, human_size(ui, f.size))
                        data-url=f.url.clone() on:click=move |_| choose(url.clone())>
                        <div class="asset-thumb" class:is-file=kind != MediaKind::Image>
                            {if kind == MediaKind::Image {
                                view! { <img src=src alt="" loading="lazy" decoding="async" draggable="false"/> }.into_any()
                            } else {
                                view! { <span class="asset-icon">{kind_icon(kind)}</span> }.into_any()
                            }}
                        </div>
                        <span class="picker-name">{name}</span>
                    </button>
                }
            };
            let has_folders = !folders.is_empty();
            view! {
                <div class="overlay picker-overlay" on:mousedown=move |e| { if e.target() == e.current_target() { close() } }
                    on:dragover=move |e: leptos::ev::DragEvent| { if crate::files::has_files(&e) { e.prevent_default() } }
                    on:drop=move |e: leptos::ev::DragEvent| {
                        if crate::files::has_files(&e) {
                            e.prevent_default();
                            import(crate::files::list(e.data_transfer().and_then(|dt| dt.files())));
                        }
                    }>
                    <div class="dialog picker" role="dialog" aria-modal="true" aria-labelledby="picker-title">
                        <header class="picker-head">
                            <h2 id="picker-title">{ui.t(if images { "picker-image-title" } else { "picker-media-title" })}</h2>
                            <button class="icon-btn icon-btn--small" title=ui.t("action-close") aria-label=ui.t("action-close")
                                on:click=move |_| close()>{icons::close()}</button>
                        </header>
                        <div class="picker-tools">
                            <Search value=q key="assets-search"/>
                            {has_folders.then(|| view! {
                                <label class="mini-select" title=ui.t("picker-folder")>
                                    {icons::folder()}
                                    <select aria-label=ui.t("picker-folder")
                                        on:change=move |e| {
                                            let v = event_target_value(&e);
                                            folder.set((!v.is_empty()).then_some(v));
                                        }>
                                        <option value="">{ui.t("picker-all-folders")}</option>
                                        {folders.iter().map(|f| {
                                            let depth = f.matches('/').count();
                                            let label = format!("{}{}", "\u{a0}\u{a0}".repeat(depth), name_of(f));
                                            view! { <option value=f.clone()>{label}</option> }
                                        }).collect_view()}
                                    </select>
                                </label>
                            })}
                        </div>
                        <div class="picker-scroll">
                            {move || {
                                let v = shown.get();
                                if v.is_empty() {
                                    return loaded.get().then(|| view! {
                                        <p class="panel-empty muted">{ui.t(if files.with(Vec::is_empty) { "media-none" } else { "media-no-match" })}</p>
                                    }).into_any();
                                }
                                view! { <div class="asset-grid picker-grid">{v.into_iter().map(tile).collect_view()}</div> }.into_any()
                            }}
                        </div>
                        <footer class="picker-foot">
                            <span class="muted">{move || ui.tx(if images { "picker-count-images" } else { "media-count" }, &[("count", shown.with(Vec::len).to_string())])}</span>
                            <span class="toolbar-spacer"></span>
                            <button type="button" class="btn btn--small" on:click=move |_| crate::files::pick(accept, true, import)>
                                {icons::upload()}{ui.t("assets-import")}</button>
                        </footer>
                    </div>
                </div>
            }
        })
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
    // Composants (identifiant, utilisations, nom), par nom.
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
            let mut v: Vec<(String, usize, String)> = d
                .components
                .iter()
                .map(|c| {
                    (
                        c.id.clone(),
                        uses.get(&c.id).copied().unwrap_or(0),
                        crate::names::component_name(c),
                    )
                })
                .collect();
            v.sort_by_key(|(_, _, l)| l.to_lowercase());
            v
        })
    });
    let count_in = move |folder: &str| {
        sh.shelf.with(|s| {
            all.with(|a| {
                a.iter()
                    .filter(|(id, _, _)| {
                        within(folder_of(s, id), folder) && !folder_of(s, id).is_empty()
                    })
                    .count()
            })
        })
    };
    // Glisser une ligne : sur un dossier pour la ranger, sur la page pour y placer le composant.
    let drag = crate::insert::CanvasDrag::new(ui, ed);
    let folder_under = |cx: f64, cy: f64| -> Option<String> {
        let el = document().element_from_point(cx as f32, cy as f32)?;
        if el.closest(".component-row").ok().flatten().is_some() {
            return None;
        }
        el.closest("[data-drop-folder]")
            .ok()??
            .get_attribute("data-drop-folder")
    };
    let row = move |(id, n, label): (String, usize, String), depth: usize| {
        let item = StoredValue::new(id.clone());
        let name = StoredValue::new(label.clone());
        let menu = move |e: leptos::ev::MouseEvent| {
            use crate::ctxmenu::{CtxItem, open};
            let mut items = vec![
                CtxItem::new(ui.now("component-edit", &[]), move || {
                    crate::components::edit_component(ui, ed, &item.get_value())
                })
                .icon(icons::edit),
                CtxItem::new(ui.now("media-insert", &[]), move || {
                    insert_component(ui, ed, &item.get_value(), &name.get_value())
                })
                .icon(icons::plus),
                CtxItem::new(ui.now("component-rename", &[]), move || {
                    crate::components::rename(ui, ed, &item.get_value())
                })
                .icon(icons::text),
            ];
            items.extend(sh.move_items(&item.get_value()));
            items.push(
                CtxItem::new(ui.now("component-delete", &[]), move || {
                    crate::components::delete(ui, ed, &item.get_value())
                })
                .icon(icons::trash)
                .danger()
                .disabled(n > 0)
                .sep(),
            );
            open(ui, &e, items);
        };
        let drag_item = move || component_drag_item(&item.get_value(), &name.get_value());
        view! {
            <div class="list-row component-row" data-component=id.clone()
                style=format!("--depth:{depth}")
                title=move || format!("{}\n{}", item.get_value(), ui.tx("components-uses", &[("count", n.to_string())]))
                class:is-dragging=move || sh.dragging.get().as_deref() == Some(item.get_value().as_str())
                on:contextmenu=menu
                on:dblclick=move |_| crate::components::edit_component(ui, ed, &item.get_value())
                on:pointerdown=move |e: leptos::ev::PointerEvent| drag.down(&e)
                on:pointermove=move |e: leptos::ev::PointerEvent| {
                    if drag.moved(&e, drag_item()) {
                        if sh.dragging.get_untracked().is_none() {
                            sh.dragging.set(Some(item.get_value()));
                        }
                        let f = folder_under(f64::from(e.client_x()), f64::from(e.client_y()));
                        if sh.drop_on.get_untracked() != f {
                            sh.drop_on.set(f);
                        }
                    }
                }
                on:pointerup=move |e: leptos::ev::PointerEvent| {
                    let over = folder_under(f64::from(e.client_x()), f64::from(e.client_y()));
                    sh.dragging.set(None);
                    sh.drop_on.set(None);
                    match drag.finish(Some(&e)) {
                        None => {}
                        Some((it, Some((parent, index)))) => crate::insert::insert_item(ui, ed, &it, parent, index),
                        Some((_, None)) => if let Some(f) = over { sh.put(&item.get_value(), &f) },
                    }
                }
                on:pointercancel=move |_| { drag.finish(None); sh.dragging.set(None); sh.drop_on.set(None); }>
                <span class="list-chevron"></span>
                <span class="list-icon">{icons::component()}</span>
                <span class="list-label">{label}</span>
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
        all: Memo<Vec<(String, usize, String)>>,
        count_in: impl Fn(&str) -> usize + Copy + Send + Sync + 'static,
        row: impl Fn((String, usize, String), usize) -> AnyView + Copy + Send + Sync + 'static,
    ) -> AnyView {
        let subs = sh.shelf.with(|s| children(s, &folder));
        let items: Vec<(String, usize, String)> = sh.shelf.with(|s| {
            all.with(|a| {
                a.iter()
                    .filter(|(id, _, _)| folder_of(s, id) == folder)
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
    let row_any = move |it: (String, usize, String), depth: usize| row(it, depth).into_any();
    let found = move || {
        let ql = q.get().trim().to_lowercase();
        all.with(|a| {
            a.iter()
                .filter(|(id, _, l)| {
                    id.to_lowercase().contains(&ql) || l.to_lowercase().contains(&ql)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    };
    let root_drop = sh.drop_zone(String::new());
    // Composant à partir de la sélection : un élément (pas déjà une utilisation).
    let can_make = move || {
        ed.selected_target().is_some_and(|at| {
            ed.doc.with(|d| {
                d.as_ref()
                    .and_then(|d| revel_edit::address::node(d, &at).ok())
                    .is_some_and(|n| !matches!(n, Node::Use { .. }))
            })
        })
    };
    view! {
        <div class="panel-list components">
            <Search value=q key="components-search"/>
            <div class="list-scroll" data-drop-folder=""
                class:is-drop=move || sh.drop_on.get().as_deref() == Some("")
                on:dragover=move |e: leptos::ev::DragEvent| root_drop(e, false)
                on:drop=move |e: leptos::ev::DragEvent| root_drop(e, true)>
                {move || if q.with(|q| q.trim().is_empty()) {
                    level(sh, String::new(), 0, closed, all, count_in, row_any)
                } else {
                    found().into_iter().map(|it| row(it, 0)).collect_view().into_any()
                }}
                {move || all.with(Vec::is_empty).then(|| view! {
                    <p class="panel-empty muted">{ui.t("components-none")}</p>
                })}
            </div>
            <footer class="panel-foot">
                <span class="panel-foot-text muted">{move || ui.tx("components-count", &[("count", all.with(Vec::len).to_string())])}</span>
                <button class="icon-btn icon-btn--small" title=ui.t("library-new-folder") aria-label=ui.t("library-new-folder")
                    on:click=move |_| { q.set(String::new()); sh.creating.set(Some(String::new())) }>{icons::folder_plus()}</button>
                <button class="icon-btn icon-btn--small" title=ui.t("components-new") aria-label=ui.t("components-new")
                    disabled=move || !can_make()
                    on:click=move |_| crate::components::make_from_selection(ui, ed)>{icons::plus()}</button>
            </footer>
        </div>
    }
}

/// Élément glissé pour un composant : une utilisation.
fn component_drag_item(id: &str, label: &str) -> crate::editor::DragItem {
    crate::editor::DragItem {
        preset: revel_edit::make::Preset::Container,
        text_key: "",
        label: label.to_string(),
        tag: "div",
        block: true,
        node: Some(Node::Use {
            id: None,
            component: id.to_string(),
            args: Default::default(),
            variant: None,
            attrs: Vec::new(),
        }),
    }
}

/// Place une utilisation du composant à l'emplacement de la sélection.
fn insert_component(ui: Ui, ed: Ed, id: &str, label: &str) {
    let Some((parent, index)) = crate::insert::insertion_point(ed) else {
        ui.fail(revel_protocol::Failure::new("insert-no-place"));
        return;
    };
    crate::insert::insert_item(ui, ed, &component_drag_item(id, label), parent, index);
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
