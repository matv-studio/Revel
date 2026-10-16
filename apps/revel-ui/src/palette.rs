//! Palette de commandes (Ctrl+K, comme Figma, Framer et Webflow) : actions de l'éditeur,
//! éléments à ajouter, pages, écrans, calques de la page affichée, vues et rubriques du site.
//! Recherche sans tenir compte des accents ni de la casse ; flèches, Entrée, Échap.

use crate::editor::{Ed, Key, Selection};
use crate::{Tab, Ui, icons};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use revel_edit::make::Preset;

/// Action d'une ligne de la palette.
#[derive(Clone)]
enum Act {
    Key(&'static str, bool),
    ZoomFit,
    Zoom100,
    ZoomSel,
    Rulers,
    Guides,
    Scripts,
    SideBySide,
    Panels,
    Parent,
    Insert(Preset, &'static str),
    Page(String),
    Breakpoint(String),
    Select(Selection),
    Tab(Tab),
    Preview,
    Home,
}

#[derive(Clone)]
struct Item {
    group: &'static str,
    label: String,
    /// Raccourci ou précision affichés à droite.
    hint: String,
    icon: Option<fn() -> AnyView>,
    act: Act,
    off: bool,
}

/// Texte comparé : minuscules, sans accents.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            'ÿ' => 'y',
            'œ' => 'o',
            'æ' => 'a',
            c => c,
        })
        .collect()
}

/// Rang d'une ligne pour la recherche (`None` : écartée) : libellé qui commence par la
/// recherche, puis mot qui commence par elle, puis contenue ; tous les mots doivent y être.
fn rank(item: &Item, group: &str, q: &str) -> Option<u8> {
    if q.is_empty() {
        return Some(0);
    }
    let label = fold(&item.label);
    let hay = format!("{label} {} {}", fold(group), fold(&item.hint));
    if !q.split_whitespace().all(|w| hay.contains(w)) {
        return None;
    }
    Some(if label.starts_with(q) {
        0
    } else if label
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w.starts_with(q))
    {
        1
    } else {
        2
    })
}

fn items(ui: Ui, ed: Ed) -> Vec<Item> {
    let k = |key: &str| ui.now(key, &[]);
    let in_editor = matches!(
        ui.route.get_untracked(),
        crate::Route::Site {
            tab: Tab::Editor,
            ..
        }
    );
    let sel = ed.selected.get_untracked().is_none();
    let can = |f: fn(&revel_edit::EditState) -> bool| {
        ed.state.with_untracked(|s| s.as_ref().is_some_and(f))
    };
    let act = |group, label: String, hint: String, icon, act, off| Item {
        group,
        label,
        hint,
        icon,
        act,
        off,
    };
    let mut v: Vec<Item> = vec![
        act(
            "palette-actions",
            k("editor-undo-short"),
            k("key-undo"),
            Some(icons::undo as fn() -> AnyView),
            Act::Key("z", false),
            !can(|s| s.can_undo),
        ),
        act(
            "palette-actions",
            k("editor-redo-short"),
            k("key-redo"),
            Some(icons::redo),
            Act::Key("z", true),
            !can(|s| s.can_redo),
        ),
        act(
            "palette-actions",
            k("layers-duplicate-short"),
            k("key-duplicate"),
            Some(icons::copy),
            Act::Key("d", false),
            sel,
        ),
        act(
            "palette-actions",
            k("layers-delete-short"),
            k("key-delete"),
            Some(icons::trash),
            Act::Key("delete", false),
            sel,
        ),
        act(
            "palette-actions",
            k("layers-wrap"),
            k("key-wrap"),
            Some(icons::wrap),
            Act::Key("g", false),
            sel,
        ),
        act(
            "palette-actions",
            k("editor-unwrap"),
            k("key-unwrap"),
            Some(icons::reverse),
            Act::Key("g", true),
            sel,
        ),
        act(
            "palette-actions",
            k("editor-copy"),
            k("key-copy"),
            Some(icons::copy),
            Act::Key("c", false),
            sel,
        ),
        act(
            "palette-actions",
            k("editor-paste"),
            k("key-paste"),
            Some(icons::copy),
            Act::Key("v", false),
            ui.clipboard.with_untracked(Vec::is_empty),
        ),
        act(
            "palette-actions",
            k("editor-select-parent"),
            k("key-parent"),
            Some(icons::parent),
            Act::Parent,
            sel,
        ),
        act(
            "palette-view",
            k("editor-zoom-fit-short"),
            k("key-zoom-fit"),
            Some(icons::search),
            Act::ZoomFit,
            false,
        ),
        act(
            "palette-view",
            k("editor-zoom-100"),
            k("key-zoom-100"),
            Some(icons::search),
            Act::Zoom100,
            false,
        ),
        act(
            "palette-view",
            k("editor-zoom-selection"),
            k("key-zoom-sel"),
            Some(icons::search),
            Act::ZoomSel,
            sel,
        ),
        act(
            "palette-view",
            k("editor-rulers"),
            k("key-rulers"),
            Some(icons::ruler),
            Act::Rulers,
            false,
        ),
        act(
            "palette-view",
            k("editor-guides"),
            String::new(),
            Some(icons::ruler),
            Act::Guides,
            false,
        ),
        act(
            "palette-view",
            k("editor-side"),
            String::new(),
            Some(icons::screens),
            Act::SideBySide,
            false,
        ),
        act(
            "palette-view",
            k("editor-scripts"),
            String::new(),
            Some(icons::code),
            Act::Scripts,
            false,
        ),
        act(
            "palette-view",
            k("dock-hide-all"),
            k("key-panels"),
            None,
            Act::Panels,
            false,
        ),
    ];
    if !in_editor {
        // Hors de l'éditeur : actions de la page sans objet.
        v.retain(|i| !matches!(i.act, Act::Key(..) | Act::Parent | Act::ZoomSel));
    }
    for (_, label, text, icon, preset) in crate::insert::palette_items() {
        v.push(act(
            "palette-add",
            k(label),
            String::new(),
            Some(icon),
            Act::Insert(preset, text),
            false,
        ));
    }
    // Vues et rubriques du site.
    for (t, key) in crate::VIEWS {
        if !matches!(t, Tab::Settings(_)) {
            v.push(act(
                "palette-goto",
                k(key),
                String::new(),
                None,
                Act::Tab(*t),
                false,
            ));
        }
    }
    for (s, key, planned) in crate::site::SECTIONS {
        v.push(act(
            "palette-goto",
            format!("{} › {}", k("view-settings"), k(key)),
            String::new(),
            Some(icons::settings),
            Act::Tab(Tab::Settings(*s)),
            planned.is_some(),
        ));
    }
    v.push(act(
        "palette-goto",
        k("preview-open"),
        String::new(),
        Some(icons::play),
        Act::Preview,
        false,
    ));
    v.push(act(
        "palette-goto",
        k("nav-all-sites"),
        String::new(),
        None,
        Act::Home,
        false,
    ));
    // Pages du site.
    for p in ed.pages.get_untracked() {
        let name = crate::pagetree::title_of(&p);
        v.push(act(
            "palette-pages",
            name,
            p.url.clone(),
            Some(icons::page),
            Act::Page(p.url.clone()),
            false,
        ));
    }
    // Écrans (points de rupture).
    let bps = ed.doc.with_untracked(|d| {
        d.as_ref()
            .map(|d| d.breakpoints.clone())
            .unwrap_or_default()
    });
    for b in bps {
        v.push(act(
            "palette-screens",
            crate::styles::bp_name(ui, &b),
            String::new(),
            Some(match b.max_width {
                Some(w) if w <= 600 => icons::mobile as fn() -> AnyView,
                Some(_) => icons::tablet,
                None => icons::desktop,
            }),
            Act::Breakpoint(b.id.clone()),
            false,
        ));
    }
    // Calques de la page affichée (éléments).
    let url = ed.page.get_untracked();
    let page = ed
        .pages
        .with_untracked(|ps| ps.iter().find(|p| p.url == url).cloned());
    if let Some(page) = page {
        let rows = ed.doc.with_untracked(|d| {
            d.as_ref()
                .map(|d| crate::layers::rows(d, &page))
                .unwrap_or_default()
        });
        for r in rows {
            let (Some(id), crate::layers::Kind::Element { tag, .. }) = (r.id.clone(), &r.kind)
            else {
                continue;
            };
            let label = crate::layers::label(ui, &r.kind);
            v.push(act(
                "palette-layers",
                label.clone(),
                String::new(),
                None,
                Act::Select(Selection {
                    id,
                    tag: tag.clone(),
                    label,
                }),
                false,
            ));
        }
    }
    v
}

fn run(ui: Ui, ed: Ed, dock: crate::dock::Dock, a: Act) {
    let editor = move || ui.tab(Tab::Editor);
    match a {
        Act::Key(key, shift) => {
            let k = Key {
                key: key.into(),
                code: String::new(),
                ctrl: !matches!(key, "delete"),
                shift,
                alt: false,
            };
            ed.shortcut(ui, &k);
        }
        Act::ZoomFit => ed.zoom_to(None),
        Act::Zoom100 => ed.zoom_to(Some(1.0)),
        Act::ZoomSel => ed.zoom_selection(),
        Act::Rulers => {
            let on = !ed.rulers.get_untracked();
            ed.rulers.set(on);
            crate::pref_set("rulers", if on { "1" } else { "0" });
        }
        Act::Guides => {
            ed.guides.update(|g| g.on = !g.on);
            if let Ok(j) = serde_json::to_string(&ed.guides.get_untracked()) {
                crate::pref_set("guides", &j);
            }
        }
        Act::Scripts => ed.set_scripts(ui, !ed.scripts.get_untracked()),
        Act::SideBySide => {
            editor();
            ed.set_side(!ed.side.get_untracked());
        }
        Act::Panels => dock.layout.update(|l| l.hidden = !l.hidden),
        Act::Parent => ed.post(serde_json::json!({ "type": "nav", "dir": "parent" })),
        Act::Insert(preset, text) => {
            editor();
            crate::insert::add_preset(ui, ed, preset, text);
        }
        Act::Page(url) => {
            editor();
            if ed.page.get_untracked() != url {
                ed.page.set(url);
                ed.scroll.set((0.0, 0.0));
                ed.selected.set(None);
            }
        }
        Act::Breakpoint(id) => {
            editor();
            ed.breakpoint.set(id);
            ed.recenter();
        }
        Act::Select(s) => {
            editor();
            let id = s.id.clone();
            ed.selected.set(Some(s));
            ed.post(serde_json::json!({ "type": "select", "id": id, "reveal": true }));
        }
        Act::Tab(t) => ui.tab(t),
        Act::Preview => ui.open_preview(false),
        Act::Home => ui.route.set(crate::Route::Home),
    }
}

/// Palette (Ctrl+K), dans toutes les vues du site ouvert ; Ctrl+K reste « lien » dans un texte
/// riche.
#[component]
pub fn Palette() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let dock = expect_context::<crate::dock::Dock>();
    let query = RwSignal::new(String::new());
    let cur = RwSignal::new(0usize);
    let list = RwSignal::new(Vec::<Item>::new());
    let input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if ed.palette.get() {
            list.set(items(ui, ed));
            query.set(String::new());
            request_animation_frame(move || {
                if let Some(i) = input.get_untracked() {
                    let _ = i.focus();
                }
            });
        }
    });
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        // Ctrl+K ou Ctrl+Espace.
        let space = e.key() == " " || e.code() == "Space";
        if !(e.ctrl_key() || e.meta_key())
            || !(e.key().to_lowercase() == "k" || space)
            || e.shift_key()
        {
            return;
        }
        if !matches!(ui.route.get_untracked(), crate::Route::Site { .. }) {
            return;
        }
        let rich = document()
            .active_element()
            .is_some_and(|el| el.closest("[contenteditable]").ok().flatten().is_some());
        if rich {
            return;
        }
        // Ouverture seulement (l'éditeur réagit aussi à Ctrl+K) ; Échap ferme.
        e.prevent_default();
        ed.palette.set(true);
    });
    on_cleanup(move || on_key.remove());

    // Lignes retenues, groupées dans l'ordre des familles, au plus 80.
    let shown = Signal::derive(move || {
        let q = fold(query.get().trim());
        let mut v: Vec<(u8, usize, Item)> = list.with(|l| {
            l.iter()
                .enumerate()
                .filter(|(_, it)| !(q.is_empty() && it.group == "palette-layers"))
                .filter_map(|(i, it)| {
                    rank(it, &ui.now(it.group, &[]), &q).map(|r| (r, i, it.clone()))
                })
                .collect()
        });
        if !q.is_empty() {
            v.sort_by_key(|(r, i, _)| (*r, *i));
        }
        v.into_iter()
            .take(80)
            .map(|(_, _, it)| it)
            .collect::<Vec<_>>()
    });
    // Ligne active : la première disponible, puis les flèches passent les lignes indisponibles.
    let first_on = move || shown.with(|l| l.iter().position(|it| !it.off).unwrap_or(0));
    Effect::new(move |_| {
        query.track();
        list.track();
        cur.set(first_on());
    });
    let step = move |dir: isize| {
        let offs: Vec<bool> = shown.with(|l| l.iter().map(|it| it.off).collect());
        let n = offs.len() as isize;
        if n == 0 {
            return;
        }
        let mut i = cur.get_untracked() as isize;
        for _ in 0..n {
            i = (i + dir).rem_euclid(n);
            if !offs[i as usize] {
                break;
            }
        }
        cur.set(i as usize);
    };
    let close = move || ed.palette.set(false);
    let fire = move |it: Item| {
        if it.off {
            return;
        }
        close();
        run(ui, ed, dock, it.act);
    };
    let scroll_cur = move || {
        request_animation_frame(|| {
            if let Ok(Some(el)) = document().query_selector(".palette-item.is-cur") {
                let opts = web_sys::ScrollIntoViewOptions::new();
                opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
                el.scroll_into_view_with_scroll_into_view_options(&opts);
            }
        });
    };
    let on_input_key = move |e: ev::KeyboardEvent| {
        let n = shown.with(Vec::len);
        match e.key().as_str() {
            "ArrowDown" => {
                e.prevent_default();
                if n > 0 {
                    step(1);
                    scroll_cur();
                }
            }
            "ArrowUp" => {
                e.prevent_default();
                if n > 0 {
                    step(-1);
                    scroll_cur();
                }
            }
            "Enter" => {
                e.prevent_default();
                if let Some(it) = shown.with(|l| l.get(cur.get_untracked()).cloned()) {
                    fire(it);
                }
            }
            "Escape" => {
                e.prevent_default();
                e.stop_propagation();
                close();
            }
            _ => {}
        }
    };
    view! {
        <Show when=move || ed.palette.get()>
            <div class="overlay overlay--top" on:mousedown=move |e| if e.target() == e.current_target() { close() }>
                <div class="palette" role="dialog" aria-modal="true" aria-label=ui.t("palette")>
                    <div class="palette-search">
                        {icons::search()}
                        <input class="palette-input" node_ref=input spellcheck="false" autocomplete="off"
                            placeholder=ui.t("palette-placeholder") aria-label=ui.t("palette")
                            role="combobox" aria-expanded="true" aria-controls="palette-list"
                            prop:value=move || query.get()
                            on:input=move |e| query.set(event_target_value(&e))
                            on:keydown=on_input_key/>
                        <kbd class="palette-esc">"Esc"</kbd>
                    </div>
                    <ul class="palette-list" id="palette-list" role="listbox">
                        {move || {
                            let l = shown.get();
                            if l.is_empty() {
                                return view! { <li class="palette-none muted">{ui.t("palette-none")}</li> }.into_any();
                            }
                            let mut prev = "";
                            l.into_iter().enumerate().map(|(i, it)| {
                                let head = (it.group != prev).then(|| view! {
                                    <li class="palette-group" role="presentation">{ui.t(it.group)}</li>
                                });
                                prev = it.group;
                                let it2 = it.clone();
                                view! {
                                    {head}
                                    <li class="palette-item" role="option" class:is-cur=move || cur.get() == i
                                        class:is-off=it.off aria-selected=move || (cur.get() == i).to_string()
                                        aria-disabled=it.off.to_string()
                                        on:mousemove=move |_| if cur.get_untracked() != i && !it.off { cur.set(i) }
                                        on:click=move |_| fire(it2.clone())>
                                        <span class="palette-icon">{it.icon.map(|f| f())}</span>
                                        <span class="palette-label">{it.label.clone()}</span>
                                        {(!it.hint.is_empty()).then(|| view! { <span class="palette-hint">{it.hint.clone()}</span> })}
                                    </li>
                                }
                            }).collect_view().into_any()
                        }}
                    </ul>
                </div>
            </div>
        </Show>
    }
}
