//! Zone du canevas : cadre de la page, zoom et déplacement de la vue (comme Figma et Framer),
//! élément glissé depuis le panneau « Ajouter ».
//!
//! - molette sur la page : la page défile ; molette autour du cadre : la vue se déplace ;
//! - Alt+molette (ou Ctrl+molette, pincement) : zoom autour du pointeur ;
//! - espace maintenue + glisser, ou bouton du milieu : déplacement de la vue ;
//! - clic dans le vide autour du cadre : plus de sélection ;
//! - Ctrl+R : règles en pixels de la page ; guides de colonnes par-dessus la page.
//!
//! La molette est écoutée directement sur la zone, en mode actif (`passive: false`) : un
//! écouteur délégué à la fenêtre (Leptos) y serait passif, le navigateur ignorerait alors
//! `preventDefault` (https://developer.chrome.com/blog/scrolling-intervention-2/).
//!
//! Le cadre garde sa hauteur quand le zoom change (la page n'est pas remise en page) : elle
//! correspond à la place disponible au zoom ajusté.

use crate::Ui;
use crate::editor::{Ed, Selection};
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn CanvasArea(failed: RwSignal<bool>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let area = NodeRef::<html::Div>::new();

    // Taille de la zone (fenêtre, colonnes de l'espace de travail, panneaux masqués).
    // L'observateur de taille peut encore appeler après la fermeture du site (zone retirée) :
    // valeurs déjà libérées ignorées.
    let measure = move || {
        if let Some(Some(a)) = area.try_get_untracked() {
            let _ = ed
                .area
                .try_set((f64::from(a.client_width()), f64::from(a.client_height())));
        }
    };
    let on_resize = window_event_listener(ev::resize, move |_| measure());
    Effect::new(move |_| {
        if let Some(a) = area.get() {
            let cb = wasm_bindgen::closure::Closure::<dyn Fn()>::new(measure);
            if let Ok(ro) = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
                ro.observe(&a);
            }
            cb.forget();
            measure();
        }
    });

    // Déplacement de la vue à la souris (espace maintenue ou bouton du milieu).
    let last = StoredValue::new((0.0f64, 0.0f64));
    let start_pan = move |e: &web_sys::PointerEvent| {
        // Vue verrouillée : la vue ne se déplace pas.
        if !untrack(|| ed.navigable()) {
            return;
        }
        if let Some(t) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        last.set_value((f64::from(e.client_x()), f64::from(e.client_y())));
        ed.panning.set(true);
    };
    let move_pan = move |e: &web_sys::PointerEvent| {
        if !ed.panning.get_untracked() {
            return;
        }
        let (x, y) = (f64::from(e.client_x()), f64::from(e.client_y()));
        let (lx, ly) = last.get_value();
        last.set_value((x, y));
        ed.pan_by(x - lx, y - ly);
    };
    let end_pan = move |_| ed.panning.set(false);
    on_cleanup(move || {
        on_resize.remove();
        ed.space.set(false);
        ed.panning.set(false);
    });

    // Point de la zone (pixels de l'écran) depuis un événement de la fenêtre.
    let local = move |cx: i32, cy: i32| -> Option<(f64, f64)> {
        let r = area.get_untracked()?.get_bounding_client_rect();
        Some((f64::from(cx) - r.left(), f64::from(cy) - r.top()))
    };

    let frame_style = move || {
        let g = ed.geo();
        // Position en `left`/`top`, échelle seule en `transform` (à 100 %, aucune transformation :
        // les pilotes WebDriver calculent mal les clics dans un cadre transformé). Position calée
        // sur les pixels de l'écran : un cadre décalé d'une fraction de pixel est flou.
        format!(
            "left:{}px;top:{}px;width:{}px;height:{}px;transform:scale({})",
            snap(g.x),
            snap(g.y),
            g.w,
            g.h,
            g.s
        )
    };
    let shield =
        move || (ed.space.get() && ed.navigable()) || ed.panning.get() || ed.drag.get().is_some();

    // Changement de taille d'écran (point de rupture) : le cadre glisse vers sa nouvelle largeur.
    let resizing = RwSignal::new(false);
    let timer = StoredValue::new(None::<leptos::leptos_dom::helpers::TimeoutHandle>);
    // Zoom en cours : cadre promu (déplacement fluide) ; 200 ms après le dernier changement,
    // promotion retirée pour que le navigateur redessine la page nette à la nouvelle échelle (sinon
    // il garde l'image dessinée à l'ancienne échelle, étirée ou réduite : texte flou).
    let zooming = RwSignal::new(false);
    let zoom_timer = StoredValue::new(None::<leptos::leptos_dom::helpers::TimeoutHandle>);
    Effect::new(move |prev: Option<f64>| {
        let s = ed.geo().s;
        if prev.is_some_and(|p| (p - s).abs() > f64::EPSILON) {
            zooming.set(true);
            if let Some(t) = zoom_timer.get_value() {
                t.clear();
            }
            zoom_timer.set_value(
                set_timeout_with_handle(
                    move || zooming.set(false),
                    std::time::Duration::from_millis(200),
                )
                .ok(),
            );
        }
        s
    });
    Effect::new(move |prev: Option<String>| {
        let bp = ed.breakpoint.get();
        if prev.is_some_and(|p| p != bp) {
            resizing.set(true);
            if let Some(t) = timer.get_value() {
                t.clear();
            }
            timer.set_value(
                set_timeout_with_handle(
                    move || resizing.set(false),
                    std::time::Duration::from_millis(420),
                )
                .ok(),
            );
        }
        bp
    });

    // Molette : écouteur actif posé sur la zone elle-même.
    Effect::new(move |_| {
        let Some(a) = area.get() else {
            return;
        };
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::WheelEvent)>::new(
            move |e: web_sys::WheelEvent| {
                e.prevent_default();
                // Vue verrouillée : la molette fait défiler la page, où que soit le pointeur.
                if !untrack(|| ed.navigable()) {
                    if !(e.alt_key() || e.ctrl_key() || e.meta_key()) {
                        ed.post(serde_json::json!({ "type": "wheel", "dx": e.delta_x(), "dy": e.delta_y() }));
                    }
                    return;
                }
                if e.alt_key() || e.ctrl_key() || e.meta_key() {
                    let at = local(e.client_x(), e.client_y());
                    // Alt+molette : certaines souris donnent un défilement horizontal.
                    let d = if e.delta_y() != 0.0 {
                        e.delta_y()
                    } else {
                        e.delta_x()
                    };
                    ed.zoom_by((-d * 0.002).exp(), at);
                } else if e.shift_key() && e.delta_x() == 0.0 {
                    ed.pan_by(-e.delta_y(), 0.0);
                } else {
                    ed.pan_by(-e.delta_x(), -e.delta_y());
                }
            },
        );
        let opts = web_sys::AddEventListenerOptions::new();
        opts.set_passive(false);
        let _ = a.add_event_listener_with_callback_and_add_event_listener_options(
            "wheel",
            cb.as_ref().unchecked_ref(),
            &opts,
        );
        cb.forget();
    });

    view! {
        <div class="canvas-area" node_ref=area
            class:is-panning=move || ed.panning.get()
            class:is-space=move || ed.space.get() && ed.navigable()
            class:is-locked=move || !ed.navigable()
            class:has-rulers=move || ed.rulers.get()
            class:is-resizing=move || resizing.get() && !ed.side.get()
            class:is-zooming=move || zooming.get()
            class:is-side=move || ed.side.get()
            class:is-comp=move || ed.comp.get().is_some()
            on:pointerdown=move |e: ev::PointerEvent| {
                if e.button() == 1 {
                    e.prevent_default();
                    start_pan(&e);
                } else if e.button() == 0 && !ed.space.get_untracked() {
                    // Clic dans le vide autour du cadre : plus de sélection.
                    let on_bg = e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        .is_some_and(|el| el.class_list().contains("canvas-area") || el.class_list().contains("canvas-world"));
                    if on_bg && ed.selected.get_untracked().is_some() {
                        ed.selected.set(None::<Selection>);
                        ed.post(serde_json::json!({ "type": "select", "id": null }));
                    }
                }
            }
            on:pointermove=move |e: ev::PointerEvent| move_pan(&e)
            on:pointerup=end_pan
            on:pointercancel=end_pan>
            <div class="canvas-world">
                <Show when=move || ed.base.get().is_some() fallback=move || view! {
                    <div class="canvas-wait muted">{move || if failed.get() { String::new() } else { ui.tx("editor-loading", &[]) }}</div>
                }>
                    {move || if ed.comp.get().is_some() {
                        view! { <ComponentFrames/> }.into_any()
                    } else if ed.side.get() {
                        view! { <SideFrames/> }.into_any()
                    } else {
                        view! {
                            <iframe class="canvas-frame is-active" data-bp=move || ed.breakpoint.get()
                                data-key=move || ed.breakpoint.get() title=ui.t("editor-canvas")
                                style=frame_style src=move || ed.src().unwrap_or_default()></iframe>
                        }.into_any()
                    }}
                    <Show when=move || ed.comp.get().is_none()>
                        <ColumnGuides/>
                    </Show>
                    // Poignées de hauteur et de largeur : canevas libre seulement.
                    <Show when=move || ed.comp.get().is_none() && ed.free.get()>
                        <FrameGrow/>
                        <Show when=move || !ed.side.get()>
                            <FrameWidth side=-1.0/>
                            <FrameWidth side=1.0/>
                        </Show>
                    </Show>
                </Show>
            </div>
            <Show when=move || ed.rulers.get()>
                <Rulers/>
            </Show>
            // Canevas des composants : retour à la page, nom du composant, transition des
            // variantes.
            {move || ed.comp.get().map(|c| view! { <ComponentBar id=c/> })}
            // Composant ouvert dans la page (modification isolée) : son nom, « Terminer ».
            {move || ed.isolated.get().filter(|_| ed.comp.get().is_none()).map(|c| {
                let name = ed.doc.with(|d| d.as_ref().and_then(|d| d.component(&c).map(crate::names::component_name))).unwrap_or(c);
                view! {
                    <div class="iso-bar" role="status">
                        <span class="iso-name">{crate::icons::component()}<span>{name}</span></span>
                        <span class="iso-sep" aria-hidden="true"></span>
                        <button class="iso-btn iso-done" on:click=move |_| ed.close_component()>{ui.t("component-done")}</button>
                    </div>
                }
            })}
            // Écran transparent au-dessus du cadre pendant un déplacement de la vue ou un glisser
            // depuis le panneau « Ajouter » : le pointeur reste dans l'interface.
            <Show when=shield>
                <div class="canvas-shield"
                    on:pointerdown=move |e: ev::PointerEvent| {
                        if ed.space.get_untracked() || e.button() == 1 {
                            e.prevent_default();
                            start_pan(&e);
                        }
                    }></div>
            </Show>
            {move || ed.drag.get().map(|d| {
                let (x, y) = ed.drag_pos.get();
                view! {
                    <div class="drag-ghost" class:is-off=move || ed.drop.get().is_none()
                        style=format!("left:{}px;top:{}px", x + 12.0, y + 14.0)>{d.label}</div>
                }
            })}
        </div>
    }
}

/// Tous les écrans côte à côte (comme Framer) : un cadre par point de rupture, du plus large au
/// plus étroit, son nom au-dessus ; un clic dans un écran (ou sur son nom) le choisit : les
/// styles s'y règlent, poignées de taille et panneaux suivent.
#[component]
fn SideFrames() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let ids = Memo::new(move |_| {
        ed.world()
            .frames
            .into_iter()
            .map(|f| f.bp)
            .collect::<Vec<_>>()
    });
    let place = move |bp: &str| {
        let w = ed.world();
        let f = w.frames.iter().find(|f| f.bp == bp).cloned()?;
        Some((w.x + f.ox * w.s, w.y, f.w, w.h, w.s))
    };
    let ring = move || {
        let bp = ed.breakpoint.get();
        place(&bp).map(|(x, y, fw, h, s)| {
            format!(
                "left:{}px;top:{}px;width:{}px;height:{}px",
                x - 2.0,
                y - 2.0,
                fw * s + 4.0,
                h * s + 4.0
            )
        })
    };
    view! {
        <div class="frame-ring" style=move || ring().unwrap_or_default() aria-hidden="true"></div>
        <For each=move || ids.get() key=|b| b.clone() let:bp>
            {
                let b = StoredValue::new(bp.clone());
                let info = move || {
                    ed.doc.with(|d| {
                        let bp = d.as_ref()?.breakpoints.iter().find(|x| x.id == b.get_value())?.clone();
                        Some(bp)
                    })
                };
                let name = move || {
                    info().map(|bp| match (bp.max_width, bp.min_width) {
                        (None, None) => {
                            let w = place(&b.get_value()).map(|p| p.2).unwrap_or_default();
                            format!("{} · {} px", crate::styles::bp_name(ui, &bp), w)
                        }
                        _ => crate::styles::bp_name(ui, &bp),
                    }).unwrap_or_default()
                };
                let icon = move || info().map(|bp| match bp.max_width {
                    Some(w) if w <= 600 => crate::icons::mobile(),
                    Some(_) => crate::icons::tablet(),
                    None => crate::icons::desktop(),
                });
                let style = move || {
                    place(&b.get_value()).map(|(x, y, fw, h, s)| {
                        format!("left:{}px;top:{}px;width:{fw}px;height:{h}px;transform:scale({s})", snap(x), snap(y))
                    }).unwrap_or_default()
                };
                let label_style = move || {
                    place(&b.get_value()).map(|(x, y, fw, _, s)| {
                        format!("left:{x}px;top:{}px;max-width:{}px", y - crate::editor::SIDE_LABEL, fw * s)
                    }).unwrap_or_default()
                };
                let on = move || ed.breakpoint.get() == b.get_value();
                view! {
                    <button class="frame-label" class:is-on=on style=label_style
                        on:click=move |_| ed.breakpoint.set(b.get_value())>{icon}<span>{name}</span></button>
                    <iframe class="canvas-frame" class:is-active=on data-bp=bp.clone() data-key=bp.clone() title=ui.t("editor-canvas")
                        style=style src=move || ed.src().unwrap_or_default()></iframe>
                }
            }
        </For>
    }
}

/// Canevas des composants (comme Framer) : une variante par cadre (principale, survol, appui,
/// personnalisées), son nom au-dessus ; clic dans un cadre ou sur son nom : variante choisie (les
/// styles s'y règlent) ; « + » ajoute une variante ; clic droit sur un nom : renommer, retirer.
#[component]
fn ComponentFrames() -> impl IntoView {
    use revel_model::VariantKind;
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let comp = move || ed.comp.get().unwrap_or_default();
    let frames = Memo::new(move |_| ed.comp_frames(&comp()));
    let place = move |key: &str| {
        let w = ed.world();
        let f = w.frames.iter().find(|f| f.bp == key).cloned()?;
        Some((
            w.x + f.ox * w.s,
            w.y + f.oy * w.s,
            f.w,
            ed.comp_frame_h(),
            w.s,
        ))
    };
    let ring = move || {
        let key = ed.active_key();
        place(&key).map(|(x, y, fw, h, s)| {
            format!(
                "left:{}px;top:{}px;width:{}px;height:{}px",
                x - 2.0,
                y - 2.0,
                fw * s + 4.0,
                h * s + 4.0
            )
        })
    };
    // Bouton « + » : à droite du dernier nom (côte à côte) ou sous le dernier cadre.
    let plus_style = move || {
        let w = ed.world();
        let last = w.frames.last()?.clone();
        let stacked = w.frames.len() > 1 && w.frames[1].oy > 0.0 || last.w > 700.0;
        let (x, y) = if stacked {
            (
                w.x + last.ox * w.s,
                w.y + (last.oy + ed.comp_frame_h()) * w.s + 12.0,
            )
        } else {
            (
                w.x + (last.ox + last.w) * w.s + 16.0,
                w.y - crate::editor::SIDE_LABEL,
            )
        };
        Some(format!("left:{x}px;top:{y}px"))
    };
    let add_menu = move |e: leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open};
        let id = comp();
        let (i1, i2, i3, i4) = (id.clone(), id.clone(), id.clone(), id.clone());
        // Survol et appui déjà là : coche (le choix montre la variante).
        let has = |k: VariantKind| {
            ed.doc.with_untracked(|d| {
                d.as_ref()
                    .and_then(|d| d.component(&id))
                    .is_some_and(|c| c.variants.iter().any(|v| v.kind == k))
            })
        };
        let mark = |on: bool| {
            if on {
                crate::icons::check as fn() -> leptos::prelude::AnyView
            } else {
                crate::icons::bolt
            }
        };
        let (hover_on, press_on) = (has(VariantKind::Hover), has(VariantKind::Pressed));
        open(
            ui,
            &e,
            vec![
                CtxItem::new(ui.now("variant-add-hover", &[]), move || {
                    crate::components::add_variant(ui, ed, &i1, VariantKind::Hover, false)
                })
                .icon(mark(hover_on)),
                CtxItem::new(ui.now("variant-add-pressed", &[]), move || {
                    crate::components::add_variant(ui, ed, &i2, VariantKind::Pressed, false)
                })
                .icon(mark(press_on)),
                // Variante prise au clic sur le composant, le clic suivant revient (D-121).
                CtxItem::new(ui.now("variant-add-click", &[]), move || {
                    crate::components::add_variant(ui, ed, &i4, VariantKind::Custom, true)
                })
                .icon(crate::icons::pointer)
                .sep(),
                CtxItem::new(ui.now("variant-add-custom", &[]), move || {
                    crate::components::add_variant(ui, ed, &i3, VariantKind::Custom, false)
                })
                .icon(crate::icons::component),
            ],
        );
    };
    view! {
        <div class="frame-ring" style=move || ring().unwrap_or_default() aria-hidden="true"></div>
        <For each=move || frames.get() key=|(k, v)| (k.clone(), format!("{v:?}")) let:item>
            {
                let (key, variant) = item;
                let k = StoredValue::new(key.clone());
                let v = StoredValue::new(variant.clone());
                let name = crate::components::variant_label(ui, variant.as_ref());
                let style = move || {
                    place(&k.get_value()).map(|(x, y, fw, h, s)| {
                        format!("left:{}px;top:{}px;width:{fw}px;height:{h}px;transform:scale({s})", snap(x), snap(y))
                    }).unwrap_or_default()
                };
                let label_style = move || {
                    place(&k.get_value()).map(|(x, y, fw, _, s)| {
                        format!("left:{x}px;top:{}px;max-width:{}px", y - crate::editor::SIDE_LABEL, fw * s)
                    }).unwrap_or_default()
                };
                let on = move || ed.active_key() == k.get_value();
                let src = move || {
                    let base = ed.base.get()?;
                    let page = ed.localized(&ed.page.get());
                    let enc = |t: &str| js_sys::encode_uri_component(t).as_string().unwrap_or_default();
                    let extra = v.with_value(|v| match v.as_ref().map(|v| (v.kind, v.id.clone())) {
                        Some((VariantKind::Custom, id)) => format!("&variant={}", enc(&id)),
                        Some((VariantKind::Hover, _)) => "&rvstate=hover".into(),
                        Some((VariantKind::Pressed, _)) => "&rvstate=active".into(),
                        None => String::new(),
                    });
                    Some(format!(
                        "{base}__revel/component/?component={}&page={}{extra}&rv={}",
                        enc(&comp()), enc(&page), ed.load.get()
                    ))
                };
                let menu = move |e: leptos::ev::MouseEvent| {
                    use crate::ctxmenu::{CtxItem, open};
                    e.prevent_default();
                    let Some(var) = v.get_value() else { return };
                    let id = comp();
                    let (i1, i2, vid1, vid2) = (id.clone(), id.clone(), var.id.clone(), var.id.clone());
                    let mut items = Vec::new();
                    if var.kind == VariantKind::Custom {
                        items.push(CtxItem::new(ui.now("variant-rename", &[]), move || crate::components::rename_variant(ui, ed, &i1, &vid1)).icon(crate::icons::edit));
                        // Passage à la variante au clic sur le composant (flèche dans le canevas).
                        let on = ed.doc.with_untracked(|d| d.as_ref().and_then(|d| d.component(&id)).is_some_and(|c| crate::components::variant_on_click(c, &var.id)));
                        let (i5, vid5) = (id.clone(), var.id.clone());
                        items.push(CtxItem::new(ui.now("variant-on-click", &[]), move || crate::components::set_variant_click(ui, ed, &i5, &vid5, !on))
                            .icon(if on { crate::icons::check } else { crate::icons::pointer }));
                    }
                    items.push(CtxItem::new(ui.now("variant-remove", &[]), move || crate::components::remove_variant(ui, ed, &i2, &vid2)).icon(crate::icons::trash).danger().sep());
                    open(ui, &e, items);
                };
                view! {
                    <button class="frame-label" class:is-on=on style=label_style data-variant=key.clone()
                        on:click=move |_| ed.activate_key(&k.get_value())
                        on:contextmenu=menu>{crate::icons::component()}<span>{name}</span></button>
                    <iframe class="canvas-frame" class:is-active=on data-key=key.clone() data-bp=move || ed.breakpoint.get()
                        title=ui.t("editor-canvas") style=style src=move || src().unwrap_or_default()></iframe>
                }
            }
        </For>
        <VariantArrows/>
        <button class="frame-add" style=move || plus_style().unwrap_or_default() title=ui.t("variant-add")
            aria-label=ui.t("variant-add") on:click=add_menu>{crate::icons::plus()}</button>
    }
}

/// Liens entre variantes (D-121, dessin repris de Blender en D-125) : de la variante principale à
/// chaque variante prise par un geste (survol, appui, clic), un lien relie deux prises rondes
/// posées sur le bord des cadres ; trait vert… couleur des composants, souligné d'un contour sombre ;
/// courbe, en équerre ou droit, terminé par une flèche ou un rond (réglages de Revel). Le geste est
/// écrit dans une pastille ; clic : la variante est choisie. Une variante prise au clic revient au
/// clic suivant (flèche aux deux bouts).
#[component]
fn VariantArrows() -> impl IntoView {
    use revel_model::VariantKind;
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let links = Memo::new(move |_| {
        let id = ed.comp.get().unwrap_or_default();
        ed.doc.with(|d| {
            let Some(c) = d.as_ref().and_then(|d| d.component(&id)) else {
                return Vec::new();
            };
            c.variants
                .iter()
                .filter_map(|v| {
                    let (key, both) = match v.kind {
                        VariantKind::Hover => ("variant-arrow-hover", false),
                        VariantKind::Pressed => ("variant-arrow-pressed", false),
                        VariantKind::Custom if crate::components::variant_on_click(c, &v.id) => {
                            ("variant-arrow-click", true)
                        }
                        VariantKind::Custom => return None,
                    };
                    Some((format!("@{}", v.id), key, both))
                })
                .collect::<Vec<_>>()
        })
    });
    /// Tracé d'un lien : clé de la variante, chemin, prises (départ, arrivée), pointes (arrivée,
    /// et départ pour un aller-retour), position du geste, libellé, geste à gauche.
    #[derive(Clone, PartialEq)]
    struct Link {
        key: String,
        d: String,
        from: (f64, f64),
        to: (f64, f64),
        heads: Vec<String>,
        label_at: (f64, f64),
        label: &'static str,
        both: bool,
        left: bool,
    }
    // Pointe de flèche qui finit à `tip`, dans la direction `t` (unitaire) : chevron plein, son
    // milieu sur le trait.
    let head = |tip: (f64, f64), t: (f64, f64)| {
        let n = (-t.1, t.0);
        let (l, w, notch) = (10.0, 5.0, 7.0);
        let p = |a: f64, b: f64| (tip.0 - t.0 * a + n.0 * b, tip.1 - t.1 * a + n.1 * b);
        let (b1, m, b2) = (p(l, w), p(notch, 0.0), p(l, -w));
        format!(
            "M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}Z",
            tip.0, tip.1, b1.0, b1.1, m.0, m.1, b2.0, b2.1
        )
    };
    let paths = Memo::new(move |_| {
        let style = ui.settings.with(|s| s.variant_links.clone());
        let arrows = ui.settings.with(|s| s.variant_link_end != "dot");
        let w = ed.world();
        let Some(base) = w.frames.first().cloned() else {
            return Vec::new();
        };
        let fh = ed.comp_frame_h();
        let stacked = w.frames.len() > 1 && w.frames[1].oy > 0.0;
        let at =
            |f: &crate::editor::FrameBox| (w.x + f.ox * w.s, w.y + f.oy * w.s, f.w * w.s, fh * w.s);
        let (bx, by, bw, bh) = at(&base);
        // Rayon des prises.
        const R: f64 = 4.0;
        links.get().into_iter().enumerate().filter_map(|(i, (key, label, both))| {
            let f = w.frames.iter().find(|f| f.bp == key)?;
            let (tx, ty, tw, th) = at(f);
            let d = 22.0 + crate::editor::ARROW_LANE * i as f64;
            // Prises : bord gauche des cadres empilés, bord haut des cadres côte à côte ; sens de
            // sortie (vers l'extérieur du cadre).
            let (s, t, out) = if stacked {
                ((bx - R, by + (bh / 2.0).min(28.0)), (tx - R, ty + (th / 2.0).min(28.0)), (-1.0, 0.0))
            } else {
                ((bx + bw / 2.0, by - R), (tx + tw / 2.0, ty - R), (0.0, -1.0))
            };
            let (h0, h1) = ((s.0 + out.0 * d, s.1 + out.1 * d), (t.0 + out.0 * d, t.1 + out.1 * d));
            let (path, end_dir, start_dir, mid) = match style.as_str() {
                "square" => {
                    // Équerre aux coins arrondis.
                    let r = 8.0_f64.min(d / 2.0);
                    let along = |a: (f64, f64), b: (f64, f64)| {
                        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                        let l = (dx * dx + dy * dy).sqrt().max(0.001);
                        (dx / l, dy / l)
                    };
                    let u1 = along(h0, h1);
                    let c = |p: (f64, f64), u: (f64, f64), k: f64| (p.0 + u.0 * k, p.1 + u.1 * k);
                    let a1 = c(h0, (-out.0, -out.1), r);
                    let a2 = c(h0, u1, r);
                    let b1 = c(h1, (-u1.0, -u1.1), r);
                    let b2 = c(h1, (-out.0, -out.1), r);
                    (
                        format!(
                            "M{:.1} {:.1}L{:.1} {:.1}Q{:.1} {:.1} {:.1} {:.1}L{:.1} {:.1}Q{:.1} {:.1} {:.1} {:.1}L{:.1} {:.1}",
                            s.0, s.1, a1.0, a1.1, h0.0, h0.1, a2.0, a2.1, b1.0, b1.1, h1.0, h1.1, b2.0, b2.1, t.0, t.1
                        ),
                        (-out.0, -out.1),
                        (-out.0, -out.1),
                        ((h0.0 + h1.0) / 2.0, (h0.1 + h1.1) / 2.0),
                    )
                }
                "straight" => {
                    // Deux segments droits par le milieu du couloir.
                    let m = ((h0.0 + h1.0) / 2.0, (h0.1 + h1.1) / 2.0);
                    let dir = |a: (f64, f64), b: (f64, f64)| {
                        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                        let l = (dx * dx + dy * dy).sqrt().max(0.001);
                        (dx / l, dy / l)
                    };
                    (
                        format!("M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}", s.0, s.1, m.0, m.1, t.0, t.1),
                        dir(m, t),
                        dir(m, s),
                        m,
                    )
                }
                _ => {
                    // Courbe de Bézier aux poignées perpendiculaires au bord (comme Blender).
                    let m = (
                        0.125 * s.0 + 0.375 * h0.0 + 0.375 * h1.0 + 0.125 * t.0,
                        0.125 * s.1 + 0.375 * h0.1 + 0.375 * h1.1 + 0.125 * t.1,
                    );
                    (
                        format!(
                            "M{:.1} {:.1}C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}",
                            s.0, s.1, h0.0, h0.1, h1.0, h1.1, t.0, t.1
                        ),
                        (-out.0, -out.1),
                        (-out.0, -out.1),
                        m,
                    )
                }
            };
            let mut heads = Vec::new();
            if arrows {
                // Pointe juste avant la prise : son milieu sur le trait.
                heads.push(head((t.0 - end_dir.0 * (R + 1.5), t.1 - end_dir.1 * (R + 1.5)), end_dir));
                if both {
                    heads.push(head((s.0 - start_dir.0 * (R + 1.5), s.1 - start_dir.1 * (R + 1.5)), start_dir));
                }
            }
            let left = stacked;
            let label_at = if stacked { (mid.0 - 6.0, mid.1) } else { (mid.0, mid.1) };
            Some(Link { key, d: path, from: s, to: t, heads, label_at, label, both, left })
        }).collect::<Vec<_>>()
    });
    view! {
        <svg class="variant-arrows" aria-hidden="true">
            {move || paths.get().into_iter().map(|l| {
                let k = l.key.clone();
                let on = move || ed.active_key() == k;
                view! {
                    <g class="variant-link" class:is-on=on data-variant=l.key.clone()>
                        <path class="variant-link-outline" d=l.d.clone()/>
                        <path class="variant-link-wire" d=l.d.clone()/>
                        {l.heads.iter().map(|h| view! { <path class="variant-link-head" d=h.clone()/> }).collect_view()}
                        <circle class="variant-socket" cx=l.from.0 cy=l.from.1 r="4"/>
                        <circle class="variant-socket" cx=l.to.0 cy=l.to.1 r="4"/>
                    </g>
                }
            }).collect_view()}
        </svg>
        {move || paths.get().into_iter().map(|l| {
            let k = l.key.clone();
            let key = l.key.clone();
            let (both, label) = (l.both, l.label);
            view! {
                <button class="variant-trigger" class:is-left=l.left data-variant=l.key.clone()
                    style=format!("left:{:.1}px;top:{:.1}px", l.label_at.0, l.label_at.1)
                    class:is-on=move || ed.active_key() == k
                    title=move || if both { ui.tx("variant-arrow-click-help", &[]) } else { ui.tx(label, &[]) }
                    on:click=move |_| ed.activate_key(&key)>
                    {if both { crate::icons::pointer() } else { crate::icons::bolt() }}<span>{ui.t(label)}</span>
                </button>
            }
        }).collect_view()}
    }
}

/// Barre du canevas des composants : retour à la page, nom du composant, transition des
/// variantes de survol et d'appui.
#[component]
fn ComponentBar(id: String) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let id = StoredValue::new(id);
    let name = move || {
        ed.doc
            .with(|d| {
                d.as_ref().and_then(|d| {
                    d.component(&id.get_value())
                        .map(crate::names::component_name)
                })
            })
            .unwrap_or_else(|| id.get_value())
    };
    let page = move || {
        let url = ed.page.get();
        ed.pages
            .with(|ps| {
                ps.iter()
                    .find(|p| p.url == url)
                    .map(crate::pagetree::title_of)
            })
            .unwrap_or(url)
    };
    view! {
        <div class="iso-bar comp-bar" role="toolbar" aria-label=ui.t("component-bar")>
            <button class="iso-btn comp-back" title=ui.t("component-back-help") on:click=move |_| ed.close_comp()>
                {crate::icons::arrow_left()}<span>{page}</span>
            </button>
            <span class="iso-sep" aria-hidden="true"></span>
            <span class="iso-name">{crate::icons::component()}<span>{name}</span></span>
            <span class="iso-sep" aria-hidden="true"></span>
            <crate::components::TransitionSelect id=id.get_value()/>
        </div>
    }
}

/// Poignée sous le cadre actif (comme Framer) : glisser vers le bas allonge les cadres pour
/// voir plus de la page, vers le haut les raccourcit ; double-clic : hauteur par défaut.
#[component]
fn FrameGrow() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    // Début du geste : hauteur des cadres et position du pointeur.
    let start = StoredValue::new(None::<(f64, f64)>);
    let style = move || {
        let g = ed.geo();
        format!(
            "left:{}px;top:{}px",
            g.x + g.w * g.s / 2.0 - 22.0,
            g.y + g.h * g.s + 6.0
        )
    };
    let down = move |e: ev::PointerEvent| {
        if e.button() != 0 {
            return;
        }
        e.prevent_default();
        e.stop_propagation();
        if let Some(t) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        let h = untrack(|| ed.geo().h);
        start.set_value(Some((h, f64::from(e.client_y()))));
    };
    let mv = move |e: ev::PointerEvent| {
        let Some((h0, y0)) = start.get_value() else {
            return;
        };
        let s = ed.scale();
        let h = (h0 + (f64::from(e.client_y()) - y0) / s).round().max(200.0);
        ed.frame_h.set(Some(h));
    };
    let up = move |_| start.set_value(None);
    view! {
        <div class="frame-grow" style=style role="separator" aria-orientation="horizontal"
            title=move || ui.tx("editor-frame-height-help", &[("height", format!("{}", ed.geo().h.round()))])
            aria-label=ui.t("editor-frame-height")
            on:pointerdown=down on:pointermove=mv on:pointerup=up on:pointercancel=up
            on:dblclick=move |_| ed.frame_h.set(None)></div>
    }
}

/// Poignées sur les bords gauche et droit du cadre (vue simple, comme Webflow) : glisser
/// élargit ou rétrécit la page des deux côtés pour voir le site à toutes les largeurs ; l'écran
/// réglé suit la largeur. Double-clic : largeur de l'écran.
#[component]
fn FrameWidth(side: f64) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    /// Glissement en cours : largeur, pointeur et position de la vue au départ, échelle.
    #[derive(Clone, Copy)]
    struct Drag {
        w0: f64,
        x0: f64,
        pan0: f64,
        s: f64,
        /// Dernière position du pointeur.
        x: f64,
        /// Largeur ajoutée en restant au bord de la zone (pixels de la page).
        extra: f64,
    }
    let drag = StoredValue::new(None::<Drag>);
    let dragging = RwSignal::new(false);
    let style = move || {
        let g = ed.geo();
        let x = if side < 0.0 {
            g.x - 14.0
        } else {
            g.x + g.w * g.s + 6.0
        };
        // Au milieu de la partie visible du cadre.
        let (_, ah) = ed.area.get();
        let visible = (g.h * g.s).min(ah - g.y).max(48.0);
        let h = 48.0;
        format!(
            "left:{x}px;top:{}px;height:{h}px",
            g.y + visible / 2.0 - h / 2.0
        )
    };
    // Largeur et position de la vue pour l'état du glissement : le bord tiré suit le pointeur,
    // l'autre bord reste en place ; la largeur ajoutée au bord de la zone fait défiler la vue.
    let apply = move |d: Drag| {
        let dx = d.x - d.x0;
        let (w, pan_x) = if side > 0.0 {
            (d.w0 + dx / d.s + d.extra, d.pan0 - d.extra * d.s)
        } else {
            (d.w0 - dx / d.s + d.extra, d.pan0 + dx)
        };
        let w = w.round().clamp(240.0, 3840.0);
        ed.frame_w.set(Some(w));
        ed.pan.update(|p| p.0 = pan_x);
        if let Some(bp) = ed.bp_for_width(w)
            && bp != ed.breakpoint.get_untracked()
        {
            ed.breakpoint.set(bp);
        }
    };
    // Pointeur au bord de la zone (ou au-delà, jusqu'au bord de l'écran) : la largeur continue
    // d'augmenter et la vue suit, d'autant plus vite que le pointeur est loin dans la marge.
    fn tick(drag: StoredValue<Option<Drag>>, side: f64, apply: impl Fn(Drag) + Copy + 'static) {
        let Some(mut d) = drag.get_value() else {
            return;
        };
        const EDGE: f64 = 36.0;
        if let Some(r) = document()
            .query_selector(".canvas-area")
            .ok()
            .flatten()
            .map(|a| a.get_bounding_client_rect())
        {
            let screen = web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(r.right());
            let depth = if side > 0.0 {
                d.x - (r.right().min(screen) - EDGE)
            } else {
                (r.left().max(0.0) + EDGE) - d.x
            };
            if depth > 0.0 && d.w0 + d.extra < 3840.0 {
                d.extra += (depth / EDGE).min(2.0) * 10.0 / d.s;
                drag.set_value(Some(d));
                apply(d);
            }
        }
        request_animation_frame(move || tick(drag, side, apply));
    }
    let down = move |e: ev::PointerEvent| {
        if e.button() != 0 {
            return;
        }
        e.prevent_default();
        e.stop_propagation();
        if let Some(t) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        // Zoom figé pendant le glissement : le cadre ne se réajuste pas sous le pointeur.
        let w = untrack(|| ed.world());
        let g = untrack(|| ed.geo());
        if untrack(|| ed.zoom.get()).is_none() {
            ed.pan.set((w.x, w.y));
            ed.zoom.set(Some(w.s));
        }
        let x = f64::from(e.client_x());
        drag.set_value(Some(Drag {
            w0: g.w,
            x0: x,
            pan0: untrack(|| ed.pan.get()).0,
            s: g.s,
            x,
            extra: 0.0,
        }));
        dragging.set(true);
        request_animation_frame(move || tick(drag, side, apply));
    };
    let mv = move |e: ev::PointerEvent| {
        let Some(mut d) = drag.get_value() else {
            return;
        };
        d.x = f64::from(e.client_x());
        drag.set_value(Some(d));
        apply(d);
    };
    let up = move |_| {
        drag.set_value(None);
        dragging.set(false);
    };
    view! {
        <div class="frame-width" class:is-left=side < 0.0 class:is-on=move || dragging.get() style=style
            role="separator" aria-orientation="vertical"
            title=move || ui.tx("editor-frame-width-help", &[("width", format!("{}", ed.geo().w.round()))])
            aria-label=ui.t("editor-frame-width")
            on:pointerdown=down on:pointermove=mv on:pointerup=up on:pointercancel=up
            on:dblclick=move |_| ed.frame_w.set(None)>
            <Show when=move || dragging.get()>
                <span class="frame-width-value">{move || format!("{} px", ed.geo().w.round())}</span>
            </Show>
        </div>
    }
}

/// Guides de colonnes par-dessus la page (comme les grilles de Figma) : bandes des colonnes,
/// à la position et à l'échelle du cadre, sans capter le pointeur.
#[component]
fn ColumnGuides() -> impl IntoView {
    let ed = expect_context::<Ed>();
    let style = move || {
        let g = ed.geo();
        format!(
            "left:{}px;top:{}px;width:{}px;height:{}px;transform:scale({})",
            snap(g.x),
            snap(g.y),
            g.w,
            g.h,
            g.s
        )
    };
    let columns = move || {
        let gd = ed.guides.get();
        if !gd.on || gd.count == 0 {
            return Vec::new();
        }
        let w = ed.geo().w;
        let margin = f64::from(gd.margin);
        let mut inner = (w - 2.0 * margin).max(0.0);
        if gd.max > 0 {
            inner = inner.min(f64::from(gd.max));
        }
        let left = (w - inner) / 2.0;
        let n = f64::from(gd.count);
        let gutter = f64::from(gd.gutter);
        let col = ((inner - (n - 1.0) * gutter) / n).max(0.0);
        (0..gd.count)
            .map(|i| (left + f64::from(i) * (col + gutter), col))
            .collect::<Vec<_>>()
    };
    view! {
        <Show when=move || ed.guides.get().on>
            <div class="column-guides" style=style aria-hidden="true">
                {move || columns().into_iter().map(|(x, w)| view! {
                    <div class="column-guide" style=format!("left:{x}px;width:{w}px")></div>
                }).collect_view()}
            </div>
        </Show>
    }
}

/// Pas des graduations : le plus petit dont les étiquettes restent espacées d'au moins `min`
/// pixels de l'écran.
fn ruler_step(scale: f64, min: f64) -> f64 {
    [
        1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0,
    ]
    .into_iter()
    .find(|s| s * scale >= min)
    .unwrap_or(10000.0)
}

/// Règles en pixels de la page (Ctrl+R) : en haut la largeur, à gauche la hauteur (défilement
/// compris) ; la page est plus claire que ses abords.
#[component]
fn Rulers() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let top = NodeRef::<html::Canvas>::new();
    let left = NodeRef::<html::Canvas>::new();
    const T: f64 = 20.0;
    let color = |name: &str| {
        window()
            .get_computed_style(&document().document_element()?)
            .ok()
            .flatten()?
            .get_property_value(name)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    Effect::new(move |_| {
        let g = ed.geo();
        let (aw, ah) = ed.area.get();
        let (_, sy) = ed.live_scroll.get();
        let _ = ui.settings.with(|s| s.theme.clone());
        let (Some(tc), Some(lc)) = (top.get(), left.get()) else {
            return;
        };
        let dpr = window().device_pixel_ratio().max(1.0);
        let ink = color("--text-muted").unwrap_or_else(|| "#888".into());
        let page = color("--surface").unwrap_or_else(|| "#222".into());
        let out = color("--chrome").unwrap_or_else(|| "#111".into());
        let step = ruler_step(g.s, 56.0);
        let minor = if (step / 5.0) * g.s >= 6.0 {
            step / 5.0
        } else {
            step
        };
        // (toile, longueur, origine écran, origine page, partie visible de la page, vertical)
        for (cv, len, o, p0, (from, to), vertical) in [
            (tc, aw, g.x, 0.0, (0.0, g.w), false),
            (lc, ah - T, g.y - T, sy, (sy, sy + g.h), true),
        ] {
            let el: &web_sys::HtmlCanvasElement = &cv;
            let (w, h) = if vertical { (T, len) } else { (len, T) };
            el.set_width((w * dpr) as u32);
            el.set_height((h * dpr) as u32);
            let _ = el.set_attribute("style", &format!("width:{w}px;height:{h}px"));
            let Some(ctx) = el
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
            else {
                continue;
            };
            let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
            if vertical {
                // Axe vertical dessiné comme l'horizontal, tourné.
                let _ = ctx.translate(T, 0.0);
                let _ = ctx.rotate(std::f64::consts::FRAC_PI_2);
            }
            ctx.set_fill_style_str(&out);
            ctx.fill_rect(0.0, 0.0, len, T);
            // Page visible (en pixels de l'écran).
            let a = o + (from - p0) * g.s;
            let b = o + (to - p0) * g.s;
            ctx.set_fill_style_str(&page);
            ctx.fill_rect(a.max(0.0), 0.0, (b.min(len) - a.max(0.0)).max(0.0), T);
            ctx.set_fill_style_str(&ink);
            ctx.set_stroke_style_str(&ink);
            ctx.set_font("10px system-ui, sans-serif");
            ctx.set_line_width(1.0);
            let first = ((p0 + (0.0 - o) / g.s) / minor).floor() * minor;
            let last = p0 + (len - o) / g.s;
            let mut v = first;
            ctx.begin_path();
            while v <= last {
                let x = (o + (v - p0) * g.s).round() + 0.5;
                let major = (v / step).round() * step == v;
                let tick = if major { T * 0.55 } else { T * 0.25 };
                // Graduation montée depuis le bord intérieur (côté page).
                let (y0, y1) = if vertical { (0.0, tick) } else { (T - tick, T) };
                ctx.move_to(x, y0);
                ctx.line_to(x, y1);
                if major {
                    let _ = ctx.fill_text(
                        &format!("{}", v as i64),
                        x + 3.0,
                        if vertical { T - 4.0 } else { 10.0 },
                    );
                }
                v += minor;
            }
            ctx.stroke();
        }
    });
    view! {
        <div class="canvas-rulers" aria-hidden="true">
            <canvas class="ruler ruler--top" node_ref=top></canvas>
            <canvas class="ruler ruler--left" node_ref=left></canvas>
            <div class="ruler-corner"></div>
        </div>
    }
}

/// Menu contextuel du canevas (clic droit sur un élément de la page), à la position donnée
/// dans la fenêtre.
pub fn open_canvas_menu(ui: Ui, ed: Ed, x: f64, y: f64) {
    use crate::ctxmenu::{CtxItem, open_at};
    use crate::icons;
    let sel = ed.selected.get_untracked().is_none();
    let clip = ui.clipboard.with_untracked(Vec::is_empty);
    let k = |key: &str| ui.now(key, &[]);
    let items = vec![
        CtxItem::new(k("editor-select-parent"), move || {
            ed.post(serde_json::json!({ "type": "nav", "dir": "parent" }))
        })
        .icon(icons::parent)
        .key(k("key-parent"))
        .disabled(sel),
        CtxItem::new(k("editor-zoom-selection"), move || ed.zoom_selection())
            .icon(icons::search)
            .key(k("key-zoom-sel"))
            .disabled(sel),
        CtxItem::new(k("editor-copy"), move || {
            ed.copy(ui, false);
        })
        .icon(icons::copy)
        .key(k("key-copy"))
        .disabled(sel)
        .sep(),
        CtxItem::new(k("editor-cut"), move || {
            ed.copy(ui, true);
        })
        .icon(icons::copy)
        .key(k("key-cut"))
        .disabled(sel),
        CtxItem::new(k("editor-paste"), move || {
            ed.paste(ui);
        })
        .icon(icons::copy)
        .key(k("key-paste"))
        .disabled(clip),
        CtxItem::new(k("layers-duplicate"), move || {
            ed.duplicate_selection(ui);
        })
        .icon(icons::copy)
        .key(k("key-duplicate"))
        .disabled(sel),
        CtxItem::new(k("layers-wrap"), move || {
            ed.wrap_selection(ui);
        })
        .icon(icons::wrap)
        .key(k("key-wrap"))
        .disabled(sel)
        .sep(),
        CtxItem::new(k("editor-unwrap"), move || {
            ed.unwrap(ui);
        })
        .icon(icons::reverse)
        .key(k("key-unwrap"))
        .disabled(sel),
        CtxItem::new(k("layers-up"), move || {
            ed.step(ui, -1);
        })
        .icon(icons::arrow_up)
        .key(k("key-up"))
        .disabled(sel),
        CtxItem::new(k("layers-down"), move || {
            ed.step(ui, 1);
        })
        .icon(icons::arrow_down)
        .key(k("key-down"))
        .disabled(sel),
        CtxItem::new(k("layers-delete"), move || {
            ed.remove_selection(ui);
        })
        .icon(icons::trash)
        .key(k("key-delete"))
        .disabled(sel)
        .danger()
        .sep(),
    ];
    let mut items = items;
    // Composants : créer à partir de l'élément ; ouvrir ou détacher une utilisation.
    let target = ed.selected_target();
    let is_use = target.as_ref().is_some_and(|at| {
        ed.doc.with_untracked(|d| {
            d.as_ref()
                .and_then(|d| revel_edit::address::node(d, at).ok())
                .is_some_and(|n| matches!(n, revel_model::Node::Use { .. }))
        })
    });
    let at = StoredValue::new(target);
    let delete = items.pop();
    if is_use {
        items.push(
            CtxItem::new(k("component-edit"), move || {
                if let Some(at) = at.get_value() {
                    crate::components::open_component(ed, &at)
                }
            })
            .icon(icons::edit)
            .sep(),
        );
        items.push(
            CtxItem::new(k("component-detach"), move || {
                if let Some(at) = at.get_value() {
                    crate::components::detach(ui, ed, at)
                }
            })
            .icon(icons::unlink),
        );
    } else {
        items.push(
            CtxItem::new(k("component-make"), move || {
                crate::components::make_from_selection(ui, ed)
            })
            .icon(icons::component)
            .disabled(sel)
            .sep(),
        );
    }
    items.extend(delete);
    open_at(ui, x, y, items, true);
}

/// Position calée sur les pixels physiques de l'écran (densité d'affichage comprise).
fn snap(v: f64) -> f64 {
    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .filter(|d| *d > 0.0)
        .unwrap_or(1.0);
    (v * dpr).round() / dpr
}
