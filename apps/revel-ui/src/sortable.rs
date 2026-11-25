//! Poignée de glisser des listes (D-200, maquettes validées le 24/11/2026) : à la place des
//! flèches « monter » et « descendre ». La liste porte `data-sort-list`, chaque ligne
//! `data-sort-row` ; la poignée se place dans la ligne. Glisser la poignée : un trait montre où la
//! ligne arrivera ; relâcher : `on_move(de, vers)`, rangs dans l'ordre affiché (vers = place finale).
//! Clavier : la poignée prend le focus, flèche haut ou bas = un rang.

use crate::{Ui, icons};
use leptos::prelude::*;
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use web_sys::Element;

struct Drag {
    from: usize,
    list: Element,
    row: Element,
    /// Ligne survolée (rang) et côté (après sa moitié).
    over: Option<(usize, bool)>,
}

thread_local! {
    static DRAG: RefCell<Option<Drag>> = const { RefCell::new(None) };
}

/// Lignes d'une liste, dans l'ordre affiché (sans celles d'une liste incluse).
fn rows(list: &Element) -> Vec<Element> {
    let Ok(all) = list.query_selector_all("[data-sort-row]") else {
        return Vec::new();
    };
    (0..all.length())
        .filter_map(|i| all.item(i)?.dyn_into::<Element>().ok())
        .filter(|r| {
            r.parent_element()
                .and_then(|p| p.closest("[data-sort-list]").ok().flatten())
                .is_some_and(|l| l == *list)
        })
        .collect()
}

fn clear(list: &Element) {
    for r in rows(list) {
        let _ = r.class_list().remove_2("is-drop-before", "is-drop-after");
    }
}

/// Ligne et liste de la poignée `el`, avec le rang de la ligne.
fn place(el: &Element) -> Option<(Element, Element, usize)> {
    let row = el.closest("[data-sort-row]").ok()??;
    let list = row.parent_element()?.closest("[data-sort-list]").ok()??;
    let i = rows(&list).iter().position(|r| *r == row)?;
    Some((row, list, i))
}

/// Poignée d'une ligne : glisser ou flèches du clavier ; `on_move((de, vers))`.
pub fn grip(ui: Ui, on_move: Callback<(usize, usize)>) -> AnyView {
    let down = move |e: leptos::ev::PointerEvent| {
        if e.button() != 0 {
            return;
        }
        let Some(el) = e
            .current_target()
            .and_then(|t| t.dyn_into::<Element>().ok())
        else {
            return;
        };
        let Some((row, list, from)) = place(&el) else {
            return;
        };
        e.prevent_default();
        let _ = el.set_pointer_capture(e.pointer_id());
        let _ = row.class_list().add_1("is-dragged");
        DRAG.with(|d| {
            *d.borrow_mut() = Some(Drag {
                from,
                list,
                row,
                over: None,
            })
        });
    };
    let moved = move |e: leptos::ev::PointerEvent| {
        DRAG.with(|d| {
            let mut d = d.borrow_mut();
            let Some(drag) = d.as_mut() else {
                return;
            };
            let doc = web_sys::window().and_then(|w| w.document());
            let hit = doc
                .and_then(|doc| doc.element_from_point(e.client_x() as f32, e.client_y() as f32))
                .and_then(|h| h.closest("[data-sort-row]").ok().flatten());
            clear(&drag.list);
            let list = rows(&drag.list);
            let Some((i, row)) =
                hit.and_then(|h| list.iter().position(|r| *r == h).map(|i| (i, h)))
            else {
                drag.over = None;
                return;
            };
            let rect = row.get_bounding_client_rect();
            // Lignes en grille (vignettes côte à côte) : côté gauche ou droit.
            let across = rect.width() < drag.list.get_bounding_client_rect().width() * 0.75;
            let after = if across {
                f64::from(e.client_x()) > rect.left() + rect.width() / 2.0
            } else {
                f64::from(e.client_y()) > rect.top() + rect.height() / 2.0
            };
            let _ = row.class_list().add_1(if after {
                "is-drop-after"
            } else {
                "is-drop-before"
            });
            drag.over = Some((i, after));
        });
    };
    let end = move |apply: bool| {
        let Some(drag) = DRAG.with(|d| d.borrow_mut().take()) else {
            return;
        };
        clear(&drag.list);
        let _ = drag.row.class_list().remove_1("is-dragged");
        let Some((i, after)) = drag.over.filter(|_| apply) else {
            return;
        };
        let mut to = if after { i + 1 } else { i };
        if drag.from < to {
            to -= 1;
        }
        if to != drag.from {
            on_move.run((drag.from, to));
        }
    };
    let key = move |e: leptos::ev::KeyboardEvent| {
        let dir: isize = match e.key().as_str() {
            "ArrowUp" => -1,
            "ArrowDown" => 1,
            _ => return,
        };
        let Some(el) = e
            .current_target()
            .and_then(|t| t.dyn_into::<Element>().ok())
        else {
            return;
        };
        let Some((_, list, from)) = place(&el) else {
            return;
        };
        e.prevent_default();
        let to = from as isize + dir;
        if to >= 0 && (to as usize) < rows(&list).len() {
            on_move.run((from, to as usize));
        }
    };
    view! {
        <span class="sort-grip" role="button" tabindex="0" title=ui.t("sort-drag") aria-label=ui.t("sort-drag")
            on:pointerdown=down on:pointermove=moved
            on:pointerup=move |_| end(true) on:pointercancel=move |_| end(false)
            on:keydown=key>{icons::grip()}</span>
    }
    .into_any()
}

/// Liste `v` avec l'élément `from` placé au rang `to`.
pub fn moved<T>(mut v: Vec<T>, from: usize, to: usize) -> Vec<T> {
    if from < v.len() && to < v.len() && from != to {
        let x = v.remove(from);
        v.insert(to, x);
    }
    v
}
