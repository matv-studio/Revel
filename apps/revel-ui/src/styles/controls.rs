//! Contrôles du panneau de styles (D-20) : libellé avec origine de la valeur, champ numérique à
//! glisser, choix à boutons, listes, interrupteur, menus.

use super::ctx::{Origin, PropValue, Sx, menu_id};
use crate::icons;
use leptos::ev;
use leptos::prelude::*;
use revel_style::values::{eval, fmt_num, length::parse_num, step};
use wasm_bindgen::JsCast;

// ------------------------------------------------------------------------------------------------
// Glisser pour changer une valeur (libellé, côté de la boîte des marges)

#[derive(Clone)]
struct Drag {
    x: f64,
    y: f64,
    moved: bool,
    start: String,
}

/// Suivi d'un glisser : seuil de 3 px avant de le considérer comme un geste (sinon c'est un clic).
#[derive(Clone, Copy)]
pub struct Scrub {
    st: StoredValue<Option<Drag>>,
}

fn scrubbing(on: bool) {
    if let Some(root) = document().document_element() {
        if on {
            let _ = root.set_attribute("data-scrub", "");
        } else {
            let _ = root.remove_attribute("data-scrub");
        }
    }
}

impl Scrub {
    pub fn new() -> Self {
        Self {
            st: StoredValue::new(None),
        }
    }
    /// Début : `start` est la valeur de départ.
    pub fn down(self, e: &ev::PointerEvent, start: String) {
        if e.button() != 0 {
            return;
        }
        if let Some(t) = e
            .current_target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            let _ = t.set_pointer_capture(e.pointer_id());
        }
        self.st.set_value(Some(Drag {
            x: f64::from(e.client_x()),
            y: f64::from(e.client_y()),
            moved: false,
            start,
        }));
    }
    /// Déplacement : (valeur de départ, distance en pixels) une fois le seuil franchi.
    /// `vertical` : vers le haut augmente aussi.
    pub fn moved(self, e: &ev::PointerEvent, vertical: bool) -> Option<(String, f64)> {
        let mut out = None;
        self.st.update_value(|d| {
            if let Some(d) = d {
                let dx = f64::from(e.client_x()) - d.x;
                let dy = f64::from(e.client_y()) - d.y;
                if !d.moved && dx.abs() + dy.abs() < 3.0 {
                    return;
                }
                if !d.moved {
                    scrubbing(true);
                }
                d.moved = true;
                out = Some((d.start.clone(), if vertical { dx - dy } else { dx }));
            }
        });
        out
    }
    /// Fin : `Some(true)` après un glisser, `Some(false)` pour un simple clic.
    pub fn up(self) -> Option<bool> {
        let d = self.st.get_value()?;
        self.st.set_value(None);
        scrubbing(false);
        Some(d.moved)
    }
}

/// Multiplicateur des modificateurs : Maj ×10, Alt ×0,1.
pub fn mult(shift: bool, alt: bool) -> f64 {
    if shift {
        10.0
    } else if alt {
        0.1
    } else {
        1.0
    }
}

/// Pas d'une unité : plus fin pour `em`, `rem`, `fr`.
fn unit_step(value: &str, step: f64) -> f64 {
    match parse_num(value).map(|n| n.unit) {
        Some(u) if matches!(u.as_str(), "em" | "rem" | "fr") => step.min(0.05),
        _ => step,
    }
}

/// Valeur après un glisser de `px` pixels depuis `start`.
pub fn dragged(start: &str, px: f64, step_size: f64, unit: &str, m: f64) -> Option<String> {
    let s = unit_step(start, step_size);
    step(start, "", px.round() * s * m, unit)
}

/// Valeur de départ d'un glisser ou d'une flèche : réglée ici, héritée, effective si elle est
/// dans l'unité attendue, sinon `zero`.
pub fn start_value(pv: &PropValue, unit: &str, zero: &str) -> String {
    for v in [pv.here.as_ref(), pv.inherited.as_ref()]
        .into_iter()
        .flatten()
    {
        if parse_num(v).is_some() {
            return v.clone();
        }
    }
    if parse_num(&pv.computed).is_some_and(|n| n.unit == unit) {
        return pv.computed.clone();
    }
    zero.to_string()
}

// ------------------------------------------------------------------------------------------------
// Libellé : origine de la valeur, menu (origine, réinitialiser), glisser

/// Origine combinée d'un groupe de propriétés (réglée ici si l'une l'est).
fn group_origin(sx: Sx, props: &[&str]) -> (Origin, bool) {
    let mut inherited = None;
    let mut resettable = false;
    let mut here = false;
    for p in props {
        let v = sx.get(p);
        match v.origin {
            Origin::Here => {
                here = true;
                resettable |= v.resettable;
            }
            Origin::Inherited(l) if inherited.is_none() => inherited = Some(l),
            _ => {}
        }
    }
    let o = if here {
        Origin::Here
    } else {
        inherited.map(Origin::Inherited).unwrap_or(Origin::None)
    };
    (o, resettable)
}

/// Retire les valeurs réglées ici.
pub fn reset(sx: Sx, props: &[&'static str]) {
    sx.set_many(props.iter().map(|p| (p.to_string(), None)).collect(), false);
}

/// Glisser rattaché à un libellé.
#[derive(Clone, Copy)]
pub struct LabelScrub {
    pub prop: &'static str,
    pub unit: &'static str,
    pub step: f64,
    pub zero: &'static str,
}

/// Libellé d'une propriété (ou d'un groupe), coloré comme dans Webflow : bleu = réglé ici,
/// orange = hérité. Nom CSS et aide en info-bulle. Clic : origine et bouton « réinitialiser » ;
/// Alt+clic : réinitialise ; glisser (si `scrub`) : change le nombre.
#[component]
pub fn Label(
    sx: Sx,
    props: &'static [&'static str],
    key: &'static str,
    #[prop(default = None)] scrub: Option<LabelScrub>,
) -> impl IntoView {
    let ui = sx.ui;
    let id = menu_id();
    let origin = Memo::new(move |_| group_origin(sx, props));
    let drag = Scrub::new();
    let click = move |alt: bool| {
        if alt {
            if origin.get_untracked().1 {
                reset(sx, props);
            }
        } else {
            sx.toggle_menu(id);
        }
    };
    let title = move || {
        let mut t = props.join(", ");
        t.push_str(" — ");
        t.push_str(&ui.tx(
            if scrub.is_some() {
                "styles-label-help"
            } else {
                "styles-label-help-reset"
            },
            &[],
        ));
        t
    };
    view! {
        <span class="style-label pop-trigger" class:is-here=move || origin.get().0 == Origin::Here
            class:is-inherited=move || matches!(origin.get().0, Origin::Inherited(_))
            class:can-scrub=scrub.is_some()
            title=title role="button" tabindex="-1"
            on:pointerdown=move |e: ev::PointerEvent| {
                if let Some(s) = scrub {
                    drag.down(&e, start_value(&sx.get(s.prop), s.unit, s.zero));
                }
            }
            on:pointermove=move |e: ev::PointerEvent| {
                if let Some(s) = scrub
                    && let Some((start, px)) = drag.moved(&e, false)
                    && let Some(v) = dragged(&start, px, s.step, s.unit, mult(e.shift_key(), e.alt_key()))
                {
                    sx.preview(&[(s.prop.to_string(), v.clone())]);
                    drag_last(v);
                }
            }
            on:pointerup=move |e: ev::PointerEvent| {
                if let Some(s) = scrub {
                    match drag.up() {
                        Some(true) => {
                            if let Some(v) = take_last() {
                                sx.set(s.prop, Some(v));
                            }
                        }
                        Some(false) => click(e.alt_key()),
                        None => {}
                    }
                }
            }
            on:click=move |e: ev::MouseEvent| { if scrub.is_none() { click(e.alt_key()) } }>
            <span class="style-label-text">{ui.t(key)}</span>
        </span>
        <Pop sx id>
            <div class="pop-origin">
                <p class="pop-text">{move || sx.origin_text(&origin.get().0)}</p>
                {move || (origin.get().0 == Origin::Here && origin.get().1).then(|| view! {
                    <button class="icon-btn icon-btn--small" title=ui.t("styles-reset") aria-label=ui.t("styles-reset")
                        on:click=move |_| { reset(sx, props); sx.menu.set(None); }>
                        {crate::icons::reset()}
                    </button>
                })}
            </div>
        </Pop>
    }
}

thread_local! {
    static LAST: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}
/// Dernière valeur d'aperçu d'un glisser (validée au relâchement).
pub fn drag_last(v: String) {
    LAST.with(|l| *l.borrow_mut() = Some(v));
}
pub fn take_last() -> Option<String> {
    LAST.with(|l| l.borrow_mut().take())
}

/// Ligne : libellé à gauche, contrôle à droite (ou dessous si `stack`).
#[component]
pub fn Row(
    sx: Sx,
    props: &'static [&'static str],
    key: &'static str,
    #[prop(default = None)] scrub: Option<LabelScrub>,
    #[prop(optional)] stack: bool,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="style-row" class:style-row--stack=stack data-prop=props.first().copied().unwrap_or_default()>
            <Label sx props key scrub/>
            <div class="style-control">{children()}</div>
        </div>
    }
}

/// Section repliable ; une pastille signale une valeur réglée ici quand elle est repliée.
#[component]
pub fn Section(
    sx: Sx,
    key: &'static str,
    props: &'static [&'static str],
    #[prop(optional)] open: bool,
    children: Children,
) -> impl IntoView {
    let set = Memo::new(move |_| {
        props.iter().any(|p| {
            sx.rule.with(|r| {
                r.as_ref()
                    .is_some_and(|r| revel_style::values::read(&r.decls, p).is_some())
            })
        })
    });
    view! {
        <details class="style-section" open=open>
            <summary>
                <span>{sx.ui.t(key)}</span>
                <i class="dot" class:is-on=move || set.get() aria-hidden="true"></i>
            </summary>
            <div class="style-section-body">{children()}</div>
        </details>
    }
}

// ------------------------------------------------------------------------------------------------
// Menus flottants

/// Panneau flottant ouvert par un déclencheur (`.pop-trigger`) ; fermé par un clic ailleurs ou Échap.
#[component]
pub fn Pop(
    sx: Sx,
    id: u32,
    #[prop(optional)] wide: bool,
    #[prop(optional)] narrow: bool,
    children: ChildrenFn,
) -> impl IntoView {
    // Ouvert : rendu entièrement visible dans le panneau (défilement si besoin).
    let el = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        if let Some(e) = el.get() {
            request_animation_frame(move || {
                let opts = web_sys::ScrollIntoViewOptions::new();
                opts.set_block(web_sys::ScrollLogicalPosition::Nearest);
                e.scroll_into_view_with_scroll_into_view_options(&opts);
            });
        }
    });
    view! {
        <Show when=move || sx.menu.get() == Some(id)>
            <div class="pop" class:pop--wide=wide class:pop--narrow=narrow role="dialog" node_ref=el>{children()}</div>
        </Show>
    }
}

// ------------------------------------------------------------------------------------------------
// Champ numérique

/// Unité proposée dans le menu d'un champ ; `kw` : mot-clé (valeur entière).
#[derive(Clone, Copy)]
pub struct UnitOpt {
    pub v: &'static str,
    pub key: &'static str,
    pub kw: bool,
}

const fn u(v: &'static str, key: &'static str) -> UnitOpt {
    UnitOpt { v, key, kw: false }
}
const fn k(v: &'static str, key: &'static str) -> UnitOpt {
    UnitOpt { v, key, kw: true }
}

pub const SIZE_UNITS: &[UnitOpt] = &[
    u("px", "unit-px"),
    u("%", "unit-percent"),
    u("vw", "unit-vw"),
    u("vh", "unit-vh"),
    u("em", "unit-em"),
    u("rem", "unit-rem"),
    k("auto", "unit-auto"),
    k("fit-content", "unit-fit"),
];
pub const MAX_UNITS: &[UnitOpt] = &[
    u("px", "unit-px"),
    u("%", "unit-percent"),
    u("vw", "unit-vw"),
    u("vh", "unit-vh"),
    u("em", "unit-em"),
    u("rem", "unit-rem"),
    k("none", "unit-none"),
];
pub const GAP_UNITS: &[UnitOpt] = &[
    u("px", "unit-px"),
    u("%", "unit-percent"),
    u("em", "unit-em"),
    u("rem", "unit-rem"),
    u("vw", "unit-vw"),
];
pub const FONT_UNITS: &[UnitOpt] = &[
    u("px", "unit-px"),
    u("rem", "unit-rem"),
    u("em", "unit-em"),
    u("%", "unit-percent"),
    u("vw", "unit-vw"),
];
pub const LINE_UNITS: &[UnitOpt] = &[
    u("", "unit-times"),
    u("px", "unit-px"),
    u("em", "unit-em"),
    u("%", "unit-percent"),
    k("normal", "unit-normal"),
];
pub const LETTER_UNITS: &[UnitOpt] = &[
    u("px", "unit-px"),
    u("em", "unit-em"),
    k("normal", "unit-normal"),
];
pub const PCT_UNITS: &[UnitOpt] = &[u("%", "unit-percent")];
pub const DEG_UNITS: &[UnitOpt] = &[u("deg", "unit-deg"), u("turn", "unit-turn")];
pub const TIME_UNITS: &[UnitOpt] = &[u("ms", "unit-ms"), u("s", "unit-s")];
pub const BLUR_UNITS: &[UnitOpt] = &[u("px", "unit-px"), u("em", "unit-em"), u("rem", "unit-rem")];
pub const MOVE_UNITS: &[UnitOpt] = &[
    u("px", "unit-px"),
    u("%", "unit-percent-self"),
    u("em", "unit-em"),
    u("rem", "unit-rem"),
    u("vw", "unit-vw"),
    u("vh", "unit-vh"),
];
pub const TRACK_UNITS: &[UnitOpt] = &[
    u("fr", "unit-fr"),
    u("px", "unit-px"),
    u("%", "unit-percent"),
    u("rem", "unit-rem"),
    k("auto", "unit-auto"),
    k("min-content", "unit-min-content"),
    k("max-content", "unit-max-content"),
];
/// Partage d'une valeur : nombre (sans unité) et unité, ou mot-clé entier.
fn split(v: &str) -> (String, Option<String>) {
    match parse_num(v) {
        Some(n) => (fmt_num(n.value), Some(n.unit)),
        None => (v.to_string(), None),
    }
}

/// Champ numérique : saisie (calculs, unités), flèches (Maj ×10, Alt ×0,1), glisser sur le
/// préfixe, menu d'unités expliqué. Vide : valeur retirée.
#[component]
pub fn NumBox(
    sx: Sx,
    /// Valeur réglée ici.
    #[prop(into)]
    value: Signal<Option<String>>,
    /// Indication (valeur héritée ou effective).
    #[prop(into)]
    hint: Signal<String>,
    /// Valeur de départ des flèches et du glisser.
    #[prop(into)]
    start: Signal<String>,
    on_preview: Callback<String>,
    /// Valeur validée (`None` : retirée) et fusion avec la modification précédente.
    on_commit: Callback<(Option<String>, bool)>,
    #[prop(default = "px")] unit: &'static str,
    #[prop(default = &[])] units: &'static [UnitOpt],
    #[prop(default = 1.0)] step: f64,
    /// Lettre ou mot dans le champ, à glisser (« L », « H »).
    #[prop(default = None)]
    prefix: Option<&'static str>,
    #[prop(optional)] label: &'static str,
    /// Texte fixe après la valeur (« % »).
    #[prop(default = None)]
    suffix: Option<&'static str>,
) -> impl IntoView {
    let ui = sx.ui;
    let id = menu_id();
    let with_units = !units.is_empty();
    let shown = move || {
        let v = value.get().unwrap_or_default();
        // Variable du site (`var(--step-3)`) : son nom lisible (« Step 3 »).
        if let Some(name) = var_label(&v) {
            return name;
        }
        if with_units { split(&v).0 } else { v }
    };
    // Indication arrondie au dixième (les valeurs effectives ont souvent trois décimales).
    let placeholder = move || {
        let h = hint.get();
        if let Some(name) = var_label(&h) {
            return name;
        }
        match parse_num(&h) {
            Some(n) if with_units => fmt_num((n.value * 10.0).round() / 10.0),
            Some(n) => format!("{}{}", fmt_num((n.value * 10.0).round() / 10.0), n.unit),
            // Mot-clé (« none », « auto ») : son nom court traduit.
            None => match units.iter().find(|o| o.kw && o.v == h) {
                Some(o) => ui.tx(&format!("{}-s", o.key), &[]),
                None => match h.as_str() {
                    "auto" => ui.tx("unit-auto-s", &[]),
                    "none" => ui.tx("unit-none-s", &[]),
                    _ => h,
                },
            },
        }
    };
    // Unité affichée : celle de la valeur réglée, sinon de l'indication si elle est déclarée.
    let shown_unit = move || {
        let v = value.get();
        match v.as_deref().map(split) {
            Some((_, Some(u))) => (u, true),
            Some((_, None)) => (String::new(), true),
            None => (split(&hint.get()).1.unwrap_or_default(), false),
        }
    };
    let unit_of = move || {
        value
            .get_untracked()
            .as_deref()
            .and_then(parse_num)
            .map(|n| n.unit)
            .unwrap_or_else(|| unit.to_string())
    };
    let commit_text = move |t: String, coalesce: bool| {
        let cur = value.get_untracked();
        // Nom lisible d'une variable du site : la variable.
        if cur.as_deref().and_then(var_label).as_deref() == Some(t.trim()) {
            return;
        }
        if let Some(v) = var_named(sx, &t) {
            if Some(&v) != cur.as_ref() {
                on_commit.run((Some(v), coalesce));
            }
            return;
        }
        let u = unit_of();
        let base = cur.clone().unwrap_or_else(|| start.get_untracked());
        let v = eval(&t, Some(&base), &u);
        if v == cur {
            return;
        }
        on_commit.run((v, coalesce));
    };
    let nudge = move |delta: f64| {
        let s = value
            .get_untracked()
            .unwrap_or_else(|| start.get_untracked());
        if let Some(v) = revel_style::values::step(&s, "", delta * unit_step(&s, step), &unit_of())
        {
            on_commit.run((Some(v), true));
        }
    };
    let drag = Scrub::new();
    // Pastille (D-139) : valeur fluide (`clamp()`, `min()`, `max()`) ou variable du site,
    // réglée dans sa fenêtre au lieu d'être tapée.
    let chip_id = menu_id();
    let fnv = move || value.get().as_deref().and_then(fn_parts);
    let var_name = move || {
        value.get().and_then(|v| {
            let n = v
                .trim()
                .strip_prefix("var(--")?
                .strip_suffix(')')?
                .to_string();
            (!n.contains([',', '(', ' '])).then_some(n)
        })
    };
    let chip = move || fnv().is_some() || var_name().is_some();
    let set_arg = move |i: usize, t: String| {
        let Some((name, mut args)) = fnv() else {
            return;
        };
        let t = t.trim().to_string();
        if t.is_empty() || args.get(i) == Some(&t) {
            return;
        }
        if let Some(a) = args.get_mut(i) {
            *a = t;
        }
        on_commit.run((Some(format!("{name}({})", args.join(", "))), false));
    };
    let choose = move |o: UnitOpt| {
        sx.menu.set(None);
        if o.kw {
            on_commit.run((Some(o.v.to_string()), false));
            return;
        }
        let cur = value
            .get_untracked()
            .or_else(|| Some(start.get_untracked()))
            .and_then(|v| parse_num(&v))
            .map(|n| n.value)
            .unwrap_or(if matches!(o.v, "%" | "vw" | "vh") {
                100.0
            } else {
                0.0
            });
        on_commit.run((Some(format!("{}{}", fmt_num(cur), o.v)), false));
    };
    view! {
        <div class="nbox" class:has-units=with_units class:is-here=move || value.get().is_some()>
            {prefix.map(|p| view! {
                <span class="nbox-prefix" aria-hidden="true"
                    on:pointerdown=move |e: ev::PointerEvent| drag.down(&e, value.get_untracked().unwrap_or_else(|| start.get_untracked()))
                    on:pointermove=move |e: ev::PointerEvent| {
                        if let Some((s, px)) = drag.moved(&e, false)
                            && let Some(v) = dragged(&s, px, step, &unit_of(), mult(e.shift_key(), e.alt_key()))
                        {
                            on_preview.run(v.clone());
                            drag_last(v);
                        }
                    }
                    on:pointerup=move |_| {
                        if drag.up() == Some(true) && let Some(v) = take_last() {
                            on_commit.run((Some(v), false));
                        }
                    }>{p}</span>
            })}
            {move || chip().then(|| {
                let label = match (fnv(), var_name()) {
                    (Some((name, args)), _) if name == "clamp" => format!("{} → {}", args[0], args[2]),
                    (Some((name, args)), _) => format!("{name} {}", args.join(" · ")),
                    (_, Some(n)) => var_human(&n),
                    _ => String::new(),
                };
                view! {
                    <button type="button" class="nbox-chip pop-trigger" class:is-var=move || var_name().is_some()
                        title=move || value.get().unwrap_or_default()
                        on:click=move |_| sx.toggle_menu(chip_id)>
                        {move || fnv().map(|(n, _)| view! { <span class="nbox-chip__fn">{if n == "clamp" { ui.tx("nbox-fluid", &[]) } else { n }}</span> })}
                        <span class="nbox-chip__text">{label}</span>
                    </button>
                }
            })}
            <Pop sx id=chip_id>
                {move || match (fnv(), var_name()) {
                    (Some((name, args)), _) => {
                        let keys: Vec<String> = if name == "clamp" {
                            vec![ui.tx("nbox-fn-min", &[]), ui.tx("nbox-fn-ideal", &[]), ui.tx("nbox-fn-max", &[])]
                        } else {
                            (1..=args.len()).map(|n| ui.tx("nbox-fn-value", &[("n", n.to_string())])).collect()
                        };
                        let fixed = args.get(if name == "clamp" { 1 } else { 0 }).cloned().unwrap_or_default();
                        let fixed_num = args.iter().find(|a| parse_num(a).is_some()).cloned().unwrap_or(fixed);
                        view! {
                            <div class="nbox-fn" title=ui.t("nbox-fluid-help")>
                                {args.into_iter().zip(keys).enumerate().map(|(i, (a, k))| view! {
                                    <label class="nbox-fn__row">
                                        <span class="label">{k}</span>
                                        <input class="input input--small nbox-fn__input" type="text" spellcheck="false" prop:value=a
                                            on:change=move |e| set_arg(i, event_target_value(&e))
                                            on:keydown=move |e: ev::KeyboardEvent| if e.key() == "Enter" { set_arg(i, event_target_value(&e)) }/>
                                    </label>
                                }).collect_view()}
                                <div class="nbox-fn__actions">
                                    <button type="button" class="btn btn--small" title=ui.t("nbox-fixed-help")
                                        on:click=move |_| { sx.menu.set(None); on_commit.run((Some(fixed_num.clone()), false)) }>{ui.t("nbox-fixed")}</button>
                                </div>
                            </div>
                        }.into_any()
                    }
                    (_, Some(cur)) => {
                        let vars = site_vars(sx);
                        let own = vars.iter().find(|(n, _)| *n == cur).map(|(_, v)| v.clone());
                        view! {
                            <ul class="menu-list nbox-vars" role="menu">
                                <li class="menu-title">{ui.t("nbox-vars")}</li>
                                {vars.into_iter().filter(|(_, v)| parse_num(v).is_some() || fn_parts(v).is_some()).map(|(n, v)| {
                                    let on = n == cur;
                                    let name = n.clone();
                                    view! {
                                        <li><button role="menuitemradio" class="menu-item menu-item--check" class:is-on=on aria-checked=on.to_string()
                                            on:click=move |_| { sx.menu.set(None); on_commit.run((Some(format!("var(--{name})")), false)) }>
                                            <span class="menu-check">{on.then(icons::check)}</span>
                                            <span>{var_human(&n)}</span>
                                            <span class="menu-hint mono">{v}</span>
                                        </button></li>
                                    }
                                }).collect_view()}
                                {own.map(|v| view! {
                                    <li class="menu-sep"></li>
                                    <li><button class="menu-item" title=ui.t("nbox-detach-help")
                                        on:click=move |_| { sx.menu.set(None); on_commit.run((Some(v.clone()), false)) }>
                                        <span class="menu-check">{icons::unlink()}</span><span>{ui.t("nbox-detach")}</span>
                                    </button></li>
                                })}
                            </ul>
                        }.into_any()
                    }
                    _ => ().into_any(),
                }}
            </Pop>
            <input class="nbox-input" type="text" inputmode="decimal" spellcheck="false" class:is-hidden=chip
                aria-label=move || if label.is_empty() { String::new() } else { ui.now(label, &[]) }
                prop:value=shown placeholder=placeholder
                on:change=move |e| commit_text(event_target_value(&e), false)
                on:keydown=move |e: ev::KeyboardEvent| {
                    match e.key().as_str() {
                        "Enter" => {
                            commit_text(event_target_value(&e), false);
                        }
                        "Escape" => {
                            if let Some(t) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                                t.set_value(&shown());
                                let _ = t.blur();
                            }
                        }
                        "ArrowUp" | "ArrowDown" => {
                            e.prevent_default();
                            let sign = if e.key() == "ArrowUp" { 1.0 } else { -1.0 };
                            nudge(sign * mult(e.shift_key(), e.alt_key()));
                        }
                        _ => {}
                    }
                }/>
            {suffix.map(|t| view! { <span class="nbox-suffix" aria-hidden="true">{t}</span> })}
            {with_units.then(|| view! {
                <button class="nbox-unit pop-trigger" class:is-faint=move || !shown_unit().1
                    title=ui.t("unit-menu") aria-label=ui.t("unit-menu")
                    on:click=move |_| sx.toggle_menu(id)>
                    {move || {
                        let (u, _) = shown_unit();
                        // Sans unité : « × » (fois la taille du texte) quand c'est une unité proposée.
                        if !u.is_empty() { u } else if units.iter().any(|o| o.v.is_empty()) { "×".into() } else { "–".into() }
                    }}
                </button>
                <Pop sx id narrow=true>
                    <ul class="menu-list" role="menu">
                        {units.iter().map(|o| {
                            let o = *o;
                            view! {
                                <li><button role="menuitem" class="menu-item" title=ui.t(o.key) on:click=move |_| choose(o)>
                                    {if o.v.is_empty() { "×".to_string() } else if o.kw { ui.tx(&format!("{}-s", o.key), &[]) } else { o.v.to_string() }}
                                </button></li>
                            }
                        }).collect_view()}
                    </ul>
                </Pop>
            })}
        </div>
    }
}

/// Champ numérique d'une propriété.
#[component]
pub fn PropNum(
    sx: Sx,
    prop: &'static str,
    #[prop(default = "px")] unit: &'static str,
    #[prop(default = &[])] units: &'static [UnitOpt],
    #[prop(default = 1.0)] step: f64,
    #[prop(default = "0")] zero: &'static str,
    #[prop(default = None)] prefix: Option<&'static str>,
    #[prop(optional)] label: &'static str,
) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get(prop));
    view! {
        <NumBox sx
            value=Signal::derive(move || pv.get().here)
            hint=Signal::derive(move || {
                // Écart « normal » des piles et grilles : nul (comme l'indique Webflow).
                let h = pv.get().hint();
                if h == "normal" && prop.ends_with("gap") { "0".into() } else { h }
            })
            start=Signal::derive(move || start_value(&pv.get(), unit, zero))
            on_preview=Callback::new(move |v: String| sx.preview(&[(prop.to_string(), v)]))
            on_commit=Callback::new(move |(v, co): (Option<String>, bool)| sx.set_many(vec![(prop.to_string(), v)], co))
            unit units step prefix label/>
    }
}

/// Champ numérique avec son libellé au-dessus (grilles de la taille, des ombres).
#[component]
pub fn Cell(
    sx: Sx,
    /// Une seule propriété (tranche statique pour le libellé).
    props: &'static [&'static str],
    key: &'static str,
    #[prop(default = "px")] unit: &'static str,
    #[prop(default = &[])] units: &'static [UnitOpt],
    #[prop(default = 1.0)] step: f64,
    #[prop(default = "0")] zero: &'static str,
) -> impl IntoView {
    let prop = props[0];
    view! {
        <div class="style-cell" data-prop=prop>
            <Label sx props key scrub=Some(LabelScrub { prop, unit, step, zero })/>
            <PropNum sx prop unit units step zero label=key/>
        </div>
    }
}

// ------------------------------------------------------------------------------------------------
// Choix

/// Option d'un choix : valeur écrite, libellé (clé de traduction), icône facultative, valeurs
/// équivalentes reconnues.
#[derive(Clone, Copy)]
pub struct Opt {
    pub v: &'static str,
    pub key: &'static str,
    pub icon: Option<fn() -> AnyView>,
    pub also: &'static [&'static str],
}

pub const fn o(v: &'static str, key: &'static str) -> Opt {
    Opt {
        v,
        key,
        icon: None,
        also: &[],
    }
}
pub const fn oi(v: &'static str, key: &'static str, icon: fn() -> AnyView) -> Opt {
    Opt {
        v,
        key,
        icon: Some(icon),
        also: &[],
    }
}
pub const fn oa(v: &'static str, key: &'static str, also: &'static [&'static str]) -> Opt {
    Opt {
        v,
        key,
        icon: None,
        also,
    }
}

fn norm(v: &str) -> String {
    v.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// L'option correspond-elle à la valeur ?
pub fn matches(opt: &Opt, v: &str) -> bool {
    let n = norm(v);
    n == opt.v || opt.also.iter().any(|a| *a == n)
}

/// Boutons exclusifs (texte ou icônes). L'option en vigueur sans être réglée ici est indiquée
/// plus discrètement.
#[component]
pub fn Seg(
    sx: Sx,
    prop: &'static str,
    options: &'static [Opt],
    #[prop(optional)] small: bool,
) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get(prop));
    view! {
        <div class="seg" class:seg--small=small role="radiogroup">
            {options.iter().map(|opt| {
                let opt = *opt;
                let on = move || pv.with(|p| p.here.as_deref().is_some_and(|v| matches(&opt, v)));
                let implied = move || pv.with(|p| p.here.is_none() && matches(&opt, &p.hint()));
                view! {
                    <button role="radio" class:is-on=on class:is-implied=implied
                        class:is-inherited=move || pv.with(|p| matches!(p.origin, Origin::Inherited(_)))
                        aria-checked=move || on().to_string()
                        title=move || format!("{} ({prop}: {})", ui.tx(opt.key, &[]), opt.v) aria-label=ui.t(opt.key)
                        on:click=move |_| sx.set(prop, Some(opt.v.to_string()))>
                        {match opt.icon { Some(i) => i(), None => view! { <span>{ui.t(opt.key)}</span> }.into_any() }}
                    </button>
                }
            }).collect_view()}
        </div>
    }
}

/// Ligne d'une liste : valeur écrite, libellé, valeurs équivalentes reconnues.
pub type Item = (String, String, &'static [&'static str]);

fn item_matches(it: &Item, v: &str) -> bool {
    let n = norm(v);
    norm(&it.0) == n || it.2.iter().any(|a| *a == n)
}

/// Liste déroulante ; la première ligne rappelle la valeur en vigueur quand rien n'est réglé ici.
#[component]
pub fn Choice(
    sx: Sx,
    prop: &'static str,
    #[prop(into)] options: Signal<Vec<Item>>,
) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get(prop));
    let label_of = move |v: &str| {
        options.with(|o| {
            o.iter()
                .find(|it| item_matches(it, v))
                .map(|it| it.1.clone())
        })
    };
    // Rien de réglé ici : la valeur en vigueur s'affiche dans le champ (en gris, en orange si
    // héritée), sans ligne de plus dans la liste.
    let unset = move || pv.with(|p| p.here.is_none());
    view! {
        <div class="select" class:is-unset=unset
            class:is-inherited=move || unset() && pv.with(|p| matches!(p.origin, super::ctx::Origin::Inherited(_)))>
            <select class="input" on:change=move |e| {
                let v = event_target_value(&e);
                sx.set(prop, (!v.is_empty()).then_some(v));
            }>
                <option value="" hidden=true selected=unset>{move || {
                    let h = pv.with(|p| p.hint());
                    let name = label_of(&h).unwrap_or_else(|| {
                        if prop == "font-family" {
                            sx.ed.doc.with(|d| match d.as_ref().and_then(|d| d.stylesheet.as_ref()) {
                                Some(sheet) => super::sections::font_label(sheet, &h),
                                None => revel_style::values::first_family(&h),
                            })
                        } else {
                            h
                        }
                    });
                    if name.is_empty() { ui.tx("styles-unset", &[]) } else { name }
                }}</option>
                {move || {
                    let here = pv.with(|p| p.here.clone());
                    let mut list = options.get();
                    if let Some(h) = &here && !list.iter().any(|it| item_matches(it, h)) {
                        list.push((h.clone(), h.clone(), &[]));
                    }
                    let mut done = false;
                    list.into_iter().map(|it| {
                        let sel = !done && here.as_deref().is_some_and(|h| item_matches(&it, h));
                        done |= sel;
                        view! { <option value=it.0 selected=sel>{it.1}</option> }
                    }).collect_view()
                }}
            </select>
        </div>
    }
}

/// Options traduites d'une liste fixe.
pub fn opts(sx: Sx, list: &'static [Opt]) -> Signal<Vec<Item>> {
    let ui = sx.ui;
    Signal::derive(move || {
        list.iter()
            .map(|o| (o.v.to_string(), ui.tx(o.key, &[]), o.also))
            .collect()
    })
}

/// Bouton bascule d'une valeur (`on` / `off`), icône.
#[component]
pub fn Toggle(
    sx: Sx,
    prop: &'static str,
    on: &'static str,
    off: &'static str,
    key: &'static str,
    icon: fn() -> AnyView,
) -> impl IntoView {
    let ui = sx.ui;
    let pv = Memo::new(move |_| sx.get(prop));
    let active = move || pv.with(|p| norm(&p.current()).split(' ').any(|t| t == on));
    view! {
        <button class="tool-toggle" class:is-on=move || active() && pv.with(|p| p.here.is_some())
            class:is-implied=move || active() && pv.with(|p| p.here.is_none())
            aria-pressed=move || active().to_string() title=ui.t(key) aria-label=ui.t(key)
            on:click=move |_| sx.set(prop, Some(if active() { off } else { on }.to_string()))>
            {icon()}
        </button>
    }
}

/// Interrupteur d'une valeur (`on` / `off`).
#[component]
pub fn Switch(sx: Sx, prop: &'static str, on: &'static str, off: &'static str) -> impl IntoView {
    let pv = Memo::new(move |_| sx.get(prop));
    let active = move || pv.with(|p| norm(&p.current()) == on);
    view! {
        <label class="switch">
            <input type="checkbox" prop:checked=active
                on:change=move |e| sx.set(prop, Some(if event_target_checked(&e) { on } else { off }.to_string()))/>
            <span class="switch-track"></span>
        </label>
    }
}

/// Nom lisible d'une valeur faite d'une seule variable du site (`var(--step-3)` -> « Step 3 »).
pub fn var_label(v: &str) -> Option<String> {
    let name = v.trim().strip_prefix("var(--")?.strip_suffix(')')?;
    (!name.is_empty() && !name.contains([',', '(', ' '])).then(|| var_human(name))
}

/// Nom lisible d'une variable du site : `step-3` → « Step 3 », `step--1` → « Step −1 ».
pub fn var_human(name: &str) -> String {
    match name.split_once("--") {
        Some((a, b)) if !a.is_empty() && !b.is_empty() => {
            format!(
                "{} −{}",
                crate::names::humanize(a),
                crate::names::humanize(b)
            )
        }
        _ => crate::names::humanize(name),
    }
}

/// Fonction de valeur montrée en pastille : `clamp(a, b, c)`, `min(…)`, `max(…)` (nom, arguments
/// au premier niveau).
pub fn fn_parts(v: &str) -> Option<(String, Vec<String>)> {
    let v = v.trim();
    let (name, rest) = v.split_once('(')?;
    let name = name.trim().to_ascii_lowercase();
    if !matches!(name.as_str(), "clamp" | "min" | "max") {
        return None;
    }
    let inner = rest.strip_suffix(')')?;
    let mut args = Vec::new();
    let (mut depth, mut cur) = (0i32, String::new());
    for c in inner.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(cur.trim().to_string());
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    args.push(cur.trim().to_string());
    (depth == 0 && args.iter().all(|a| !a.is_empty()) && (name != "clamp" || args.len() == 3))
        .then_some((name, args))
}

/// Variables du site (déclarations de `:root`) : (nom sans `--`, valeur).
fn site_vars(sx: Sx) -> Vec<(String, String)> {
    sx.ed.doc.with_untracked(|d| {
        let Some(d) = d.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<(String, String)> = Vec::new();
        for r in d.style_rules() {
            if !matches!(r.selector.as_str(), ":root" | "html") {
                continue;
            }
            for decl in &r.decls {
                if let Some(n) = decl.prop.strip_prefix("--")
                    && !out.iter().any(|(m, _)| m == n)
                {
                    out.push((n.to_string(), decl.value.clone()));
                }
            }
        }
        out
    })
}

/// Variable du site dont le nom lisible est `text` (« Step 3 » -> `var(--step-3)`).
fn var_named(sx: Sx, text: &str) -> Option<String> {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return None;
    }
    if t.starts_with("var(") {
        return Some(text.trim().to_string());
    }
    sx.ed.doc.with_untracked(|d| {
        d.as_ref()?.style_rules().find_map(|r| {
            if !matches!(r.selector.as_str(), ":root" | "html") {
                return None;
            }
            r.decls.iter().find_map(|decl| {
                let name = decl.prop.strip_prefix("--")?;
                (crate::names::humanize(name).to_lowercase() == t).then(|| format!("var(--{name})"))
            })
        })
    })
}
