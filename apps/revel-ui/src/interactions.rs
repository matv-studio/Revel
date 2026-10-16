//! Panneau Interactions (6.1) : animations prêtes à l'emploi de l'élément sélectionné, à l'image
//! des effets de Framer. Un attribut par moment (`data-revel-appear`, `-text`, `-hover`, `-press`,
//! `-loop`, `-scroll`) : l'animation choisie puis les réglages qui diffèrent des valeurs par
//! défaut (`up dur=0.8 replay`). Le script du site (`revel-render/src/animations.js`) les lit ;
//! le même script, chargé dans l'interface, anime les vignettes des animations au survol, et dans
//! le canevas joue l'aperçu.

use crate::editor::Ed;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::{Command, Target};
use revel_model::Node;
use wasm_bindgen::JsCast;

/// Moment d'animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Appear,
    Text,
    Hover,
    Press,
    Loop,
    Scroll,
}

pub const FAMILIES: [Family; 6] = [
    Family::Appear,
    Family::Text,
    Family::Hover,
    Family::Press,
    Family::Loop,
    Family::Scroll,
];

/// Réglage proposé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Setting {
    Trigger,
    Split,
    Dur,
    Delay,
    Ease,
    Dist,
    Stagger,
    Amount,
    Mirror,
    Dir,
    Pause,
    Replay,
}

impl Family {
    pub fn id(self) -> &'static str {
        match self {
            Family::Appear => "appear",
            Family::Text => "text",
            Family::Hover => "hover",
            Family::Press => "press",
            Family::Loop => "loop",
            Family::Scroll => "scroll",
        }
    }
    pub fn attr(self) -> String {
        format!("data-revel-{}", self.id())
    }
    fn key(self) -> &'static str {
        match self {
            Family::Appear => "anim-appear",
            Family::Text => "anim-text",
            Family::Hover => "anim-hover",
            Family::Press => "anim-press",
            Family::Loop => "anim-loop",
            Family::Scroll => "anim-scroll",
        }
    }
    fn help(self) -> &'static str {
        match self {
            Family::Appear => "anim-appear-help",
            Family::Text => "anim-text-help",
            Family::Hover => "anim-hover-help",
            Family::Press => "anim-press-help",
            Family::Loop => "anim-loop-help",
            Family::Scroll => "anim-scroll-help",
        }
    }
    /// Animations proposées, dans l'ordre de la grille.
    pub fn presets(self) -> &'static [&'static str] {
        match self {
            Family::Appear => &[
                "fade", "up", "down", "left", "right", "zoom-in", "zoom-out", "blur", "pop",
                "flip", "rotate", "reveal",
            ],
            Family::Text => &["fade", "up", "blur", "pop", "flip", "typewriter"],
            Family::Hover => &[
                "grow",
                "shrink",
                "lift",
                "shadow",
                "dim",
                "tilt",
                "brighten",
                "underline",
            ],
            Family::Press => &["press", "squish", "push"],
            Family::Loop => &[
                "spin",
                "pulse",
                "float",
                "swing",
                "shake",
                "blink",
                "bounce",
                "heartbeat",
                "ticker",
            ],
            Family::Scroll => &[
                "parallax", "fade-in", "zoom-in", "rotate", "slide-x", "fade-out",
            ],
        }
    }
    /// Réglages montrés pour une animation.
    fn settings(self, preset: &str) -> Vec<Setting> {
        use Setting::*;
        match self {
            Family::Appear => {
                let mut v = vec![Trigger, Dur, Delay, Ease];
                if matches!(preset, "up" | "down" | "left" | "right" | "rotate") {
                    v.push(Dist);
                }
                v.extend([Stagger, Replay]);
                v
            }
            Family::Text if preset == "typewriter" => vec![Trigger, Stagger, Delay, Replay],
            Family::Text => vec![Split, Trigger, Dur, Stagger, Delay, Ease, Replay],
            Family::Hover => vec![Dur, Ease, Amount],
            Family::Press => vec![Dur, Amount],
            Family::Loop => match preset {
                "ticker" => vec![Dur, Dir, Pause],
                "spin" => vec![Dur, Delay, Mirror],
                "pulse" | "float" | "swing" | "blink" => vec![Dur, Delay, Ease, Amount, Mirror],
                _ => vec![Dur, Delay, Amount],
            },
            Family::Scroll => vec![Amount],
        }
    }
}

/// Valeur par défaut d'un réglage (comme le script du site).
fn default_of(f: Family, preset: &str, key: &str) -> Option<&'static str> {
    let special = match (f, preset, key) {
        (Family::Appear | Family::Text, "pop", "ease") => Some("bounce"),
        (Family::Text, "typewriter", "split") => Some("chars"),
        (Family::Text, "typewriter", "stagger") => Some("0.05"),
        (Family::Loop, "spin", "dur") => Some("4"),
        (Family::Loop, "spin", "ease") => Some("linear"),
        (Family::Loop, "shake", "dur") => Some("0.8"),
        (Family::Loop, "blink" | "bounce", "dur") => Some("1.2"),
        (Family::Loop, "heartbeat", "dur") => Some("1.6"),
        (Family::Loop, "ticker", "dur") => Some("20"),
        (Family::Loop, "ticker", "dir") => Some("left"),
        (Family::Loop, "pulse" | "float" | "swing" | "blink", "mirror") => Some("on"),
        (Family::Loop, _, "mirror") => Some("off"),
        _ => None,
    };
    if special.is_some() {
        return special;
    }
    match (f, key) {
        (Family::Appear, "dur") => Some("0.6"),
        (Family::Appear, "dist") => Some("32"),
        (Family::Appear, "stagger") => Some("0"),
        (Family::Text, "dur") => Some("0.5"),
        (Family::Text, "split") => Some("words"),
        (Family::Hover, "dur") => Some("0.25"),
        (Family::Press, "dur") => Some("0.12"),
        (Family::Loop, "dur") => Some("2"),
        (Family::Loop, "ease") => Some("ease"),
        (_, "delay") => Some("0"),
        (_, "ease") => Some("smooth"),
        (_, "trigger") => Some("view"),
        (_, "amount") => Some("100"),
        _ => None,
    }
}

/// Décalage par défaut d'un texte selon sa découpe.
fn text_stagger(split: &str) -> &'static str {
    match split {
        "chars" => "0.03",
        "lines" => "0.12",
        _ => "0.06",
    }
}

/// Valeur d'un attribut : animation, réglages (`clé=valeur`) et drapeaux (`replay`).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Anim {
    pub preset: String,
    pub opts: Vec<(String, Option<String>)>,
}

impl Anim {
    pub fn parse(v: &str) -> Anim {
        let mut it = v.split_whitespace();
        let preset = it.next().unwrap_or_default().to_string();
        let opts = it
            .map(|t| match t.split_once('=') {
                Some((k, v)) => (k.to_string(), Some(v.to_string())),
                None => (t.to_string(), None),
            })
            .collect();
        Anim { preset, opts }
    }
    pub fn write(&self) -> String {
        let mut s = self.preset.clone();
        for (k, v) in &self.opts {
            s.push(' ');
            s.push_str(k);
            if let Some(v) = v {
                s.push('=');
                s.push_str(v);
            }
        }
        s
    }
    fn get(&self, k: &str) -> Option<&str> {
        self.opts
            .iter()
            .find(|(x, _)| x == k)
            .map(|(_, v)| v.as_deref().unwrap_or(""))
    }
    fn has(&self, k: &str) -> bool {
        self.opts.iter().any(|(x, _)| x == k)
    }
    /// Réglage écrit, ou retiré (`None`, ou valeur par défaut).
    fn set(&mut self, f: Family, k: &str, v: Option<String>) {
        let v = v.filter(|v| Some(v.as_str()) != self.fallback(f, k).as_deref());
        match self.opts.iter().position(|(x, _)| x == k) {
            Some(i) => match v {
                Some(v) => self.opts[i].1 = Some(v),
                None => {
                    self.opts.remove(i);
                }
            },
            None => {
                if let Some(v) = v {
                    self.opts.push((k.to_string(), Some(v)));
                }
            }
        }
    }
    fn flag(&mut self, k: &str, on: bool) {
        self.opts.retain(|(x, _)| x != k);
        if on {
            self.opts.push((k.to_string(), None));
        }
    }
    fn fallback(&self, f: Family, k: &str) -> Option<String> {
        if f == Family::Text && k == "stagger" && self.preset != "typewriter" {
            return Some(text_stagger(self.get("split").unwrap_or("words")).to_string());
        }
        default_of(f, &self.preset, k).map(str::to_string)
    }
    /// Animation changée : réglages qui ne s'y appliquent plus retirés.
    fn with_preset(&self, f: Family, preset: &str) -> Anim {
        let keep: Vec<&str> = f.settings(preset).iter().map(|s| setting_key(*s)).collect();
        let mut a = Anim {
            preset: preset.to_string(),
            opts: self
                .opts
                .iter()
                .filter(|(k, _)| keep.contains(&k.as_str()))
                .cloned()
                .collect(),
        };
        // Valeurs devenues celles par défaut de la nouvelle animation : retirées.
        let opts = a.opts.clone();
        for (k, v) in opts {
            if v.is_some() {
                a.set(f, &k, v);
            }
        }
        a
    }
}

fn setting_key(s: Setting) -> &'static str {
    match s {
        Setting::Trigger => "trigger",
        Setting::Split => "split",
        Setting::Dur => "dur",
        Setting::Delay => "delay",
        Setting::Ease => "ease",
        Setting::Dist => "dist",
        Setting::Stagger => "stagger",
        Setting::Amount => "amount",
        Setting::Mirror => "mirror",
        Setting::Dir => "dir",
        Setting::Pause => "pause",
        Setting::Replay => "replay",
    }
}

/// Libellé d'une animation.
pub fn preset_key(f: Family, preset: &str) -> String {
    match (f, preset) {
        (Family::Scroll, "zoom-in") => "anim-p-zoom-in-scroll".into(),
        (Family::Scroll, "rotate") => "anim-p-rotate-scroll".into(),
        _ => format!("anim-p-{preset}"),
    }
}

// ------------------------------------------------------------------------------------------------
// Vignettes animées : script du site chargé dans l'interface, sans rien lancer.

fn runtime() -> Option<js_sys::Object> {
    let w = web_sys::window()?;
    js_sys::Reflect::get(&w, &"RevelAnimations".into())
        .ok()
        .filter(|v| v.is_object())
        .map(|v| v.unchecked_into())
}

fn load_runtime() {
    let Some(w) = web_sys::window() else { return };
    if runtime().is_some()
        || js_sys::Reflect::get(&w, &"__revelAnimLoading".into()).is_ok_and(|v| v.is_truthy())
    {
        return;
    }
    let _ = js_sys::Reflect::set(&w, &"RevelAnimationsManual".into(), &true.into());
    let _ = js_sys::Reflect::set(&w, &"__revelAnimLoading".into(), &true.into());
    if let Some(doc) = w.document()
        && let Ok(s) = doc.create_element("script")
    {
        let _ = s.set_attribute("src", "animations.js");
        if let Some(h) = doc.head() {
            let _ = h.append_child(&s);
        }
    }
}

/// Joue une animation sur une vignette (durées raccourcies pour les boucles).
fn play_thumb(el: &web_sys::Element, f: Family, preset: &str) {
    let Some(rt) = runtime() else { return };
    let Ok(preview) = js_sys::Reflect::get(&rt, &"preview".into()) else {
        return;
    };
    let Ok(preview) = preview.dyn_into::<js_sys::Function>() else {
        return;
    };
    let value = match (f, preset) {
        (Family::Loop, "ticker") => "ticker dur=3".to_string(),
        (Family::Loop, _) => format!("{preset} dur=0.8"),
        (Family::Appear, _) => format!("{preset} dist=10"),
        _ => preset.to_string(),
    };
    let args = js_sys::Array::of3(el, &f.id().into(), &value.into());
    let _ = preview.apply(&rt, &args);
}

/// Contenu d'une vignette : carré, texte ou rangée de points (bandeau).
fn thumb_html(f: Family, preset: &str) -> &'static str {
    match (f, preset) {
        (Family::Text, _) => "Abc",
        (Family::Hover, "underline") => "Lien",
        (Family::Loop, "ticker") => "<i></i><i></i><i></i><i></i>",
        _ => "",
    }
}

// ------------------------------------------------------------------------------------------------

/// Panneau « Interactions ».
#[component]
pub fn Interactions() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    load_runtime();
    // Élément sélectionné : recréé seulement quand la sélection change.
    let target = Memo::new(move |_| ed.selected_target());
    let kind = Memo::new(move |_| {
        let at = target.get()?;
        ed.doc.with(|d| {
            let n = revel_edit::address::node(d.as_ref()?, &at).ok()?;
            Some(match n {
                // Utilisation de composant : animation posée sur son élément racine, sans
                // l'envelopper.
                Node::Element { .. } | Node::Use { .. } => 0u8,
                Node::Image(_) => 1,
                _ => 2,
            })
        })
    });
    view! {
        <div class="interactions panel-pad">
            {move || match (target.get(), kind.get()) {
                (Some(at), Some(0)) => view! { <Families at=at/> }.into_any(),
                (Some(at), Some(1)) => view! {
                    <div class="anim-wrap">
                        <button class="btn btn--small" title=ui.t("anim-wrap-help")
                            on:click=move |_| ed.wrap(ui, at.clone())>{icons::wrap()}{ui.t("anim-wrap")}</button>
                    </div>
                }.into_any(),
                _ => view! { <p class="muted inspector-empty">{ui.t("inspector-none")}</p> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn Families(at: Target) -> impl IntoView {
    let at = StoredValue::new(at);
    view! {
        {FAMILIES.into_iter().map(|f| view! { <FamilySection f=f at=at/> }).collect_view()}
    }
}

#[component]
fn FamilySection(f: Family, at: StoredValue<Target>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let name = f.attr();
    let value = Memo::new(move |_| {
        let at = at.get_value();
        ed.doc.with(
            |d| match revel_edit::address::node(d.as_ref()?, &at).ok()? {
                Node::Element { attrs, .. } | Node::Use { attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == f.attr())
                    .and_then(|a| a.value.clone()),
                _ => None,
            },
        )
    });
    let anim = Memo::new(move |_| value.get().map(|v| Anim::parse(&v)));
    let preview = move || {
        if let Some(v) = value.get_untracked() {
            ed.post(serde_json::json!({ "type": "animate", "id": at.get_value().node, "family": f.id(), "value": v }));
        }
    };
    let write = move |a: Option<Anim>| {
        let at = at.get_value();
        let cmd = match a {
            Some(a) => Command::SetAttr {
                at,
                name: name.clone(),
                value: Some(a.write()),
            },
            None => Command::RemoveAttr {
                at,
                name: name.clone(),
            },
        };
        ed.apply(ui, cmd, false);
        // Aperçu après l'écriture (canevas rechargé si besoin).
        set_timeout(preview, std::time::Duration::from_millis(350));
    };
    let write = StoredValue::new(write);
    let add = move |_| {
        let p = f.presets()[0];
        write.with_value(|w| {
            w(Some(Anim {
                preset: p.into(),
                opts: Vec::new(),
            }))
        });
    };
    view! {
        <section class="anim-family" class:is-on=move || anim.with(Option::is_some) data-family=f.id()>
            <header class="anim-head">
                <span class="anim-title" title=ui.t(f.help())>{ui.t(f.key())}</span>
                {move || match anim.get() {
                    None => view! {
                        <button class="icon-btn icon-btn--small anim-add"
                            title=move || ui.tx("anim-add", &[("name", ui.tx(f.key(), &[]))])
                            aria-label=move || ui.tx("anim-add", &[("name", ui.tx(f.key(), &[]))])
                            on:click=add>{icons::plus()}</button>
                    }.into_any(),
                    Some(a) => view! {
                        <span class="anim-current">{ui.tx(&preset_key(f, &a.preset), &[])}</span>
                        <button class="icon-btn icon-btn--small anim-play" title=ui.t("anim-preview") aria-label=ui.t("anim-preview")
                            on:click=move |_| preview()>{icons::play()}</button>
                        <button class="icon-btn icon-btn--small anim-remove" title=ui.t("anim-remove") aria-label=ui.t("anim-remove")
                            on:click=move |_| write.with_value(|w| w(None))>{icons::trash()}</button>
                    }.into_any(),
                }}
            </header>
            <Show when=move || anim.with(Option::is_some)>
                <div class="anim-presets" role="radiogroup" aria-label=ui.t(f.key())>
                    {f.presets().iter().map(|p| {
                        let p: &'static str = p;
                        let on = move || anim.with(|a| a.as_ref().is_some_and(|a| a.preset == p));
                        let thumb = NodeRef::<leptos::html::Span>::new();
                        view! {
                            <button class="anim-tile" role="radio" class:is-on=on aria-checked=move || on().to_string()
                                data-preset=p title=move || ui.tx(&format!("anim-d-{}-{p}", f.id()), &[])
                                on:mouseenter=move |_| { if let Some(t) = thumb.get() { play_thumb(&t, f, p) } }
                                on:click=move |_| {
                                    let a = anim.get_untracked().unwrap_or_default();
                                    if a.preset != p {
                                        write.with_value(|w| w(Some(a.with_preset(f, p))));
                                    } else {
                                        preview();
                                    }
                                }>
                                <span class="anim-stage">
                                    <span class="anim-thumb" class:is-text=matches!(f, Family::Text) || p == "underline"
                                        class:is-ticker=p == "ticker" node_ref=thumb inner_html=thumb_html(f, p)></span>
                                </span>
                                <span class="anim-tile-name">{move || ui.tx(&preset_key(f, p), &[])}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
                <div class="anim-settings">
                    {move || anim.get().map(|a| {
                        f.settings(&a.preset).into_iter().map(|s| setting_row(ui, f, s, anim, write)).collect_view()
                    })}
                </div>
            </Show>
        </section>
    }
}

/// Ligne d'un réglage : libellé (bleu si réglé ici), contrôle.
fn setting_row(
    ui: Ui,
    f: Family,
    s: Setting,
    anim: Memo<Option<Anim>>,
    write: StoredValue<impl Fn(Option<Anim>) + Send + Sync + 'static>,
) -> AnyView {
    let key = setting_key(s);
    let current = move || anim.get().unwrap_or_default();
    let is_here = move || anim.with(|a| a.as_ref().is_some_and(|a| a.has(key)));
    let update = move |g: &dyn Fn(&mut Anim)| {
        let mut a = anim.get_untracked().unwrap_or_default();
        g(&mut a);
        write.with_value(|w| w(Some(a)));
    };
    let (label, help) = match s {
        Setting::Trigger => ("anim-trigger", None),
        Setting::Split => ("anim-split", None),
        Setting::Dur => (
            "anim-dur",
            Some(match f {
                Family::Loop => "anim-dur-loop-help",
                Family::Text => "anim-dur-text-help",
                _ => "anim-dur-help",
            }),
        ),
        Setting::Delay => ("anim-delay", Some("anim-delay-help")),
        Setting::Ease => ("anim-ease", None),
        Setting::Dist => ("anim-dist", Some("anim-dist-help")),
        Setting::Stagger => (
            "anim-stagger",
            Some(if f == Family::Text {
                "anim-stagger-text-help"
            } else {
                "anim-stagger-help"
            }),
        ),
        Setting::Amount => ("anim-amount", Some("anim-amount-help")),
        Setting::Mirror => ("anim-mirror", Some("anim-mirror-help")),
        Setting::Dir => ("anim-dir", None),
        Setting::Pause => ("anim-pause", None),
        Setting::Replay => ("anim-replay", Some("anim-replay-help")),
    };
    let label_view = view! {
        <span class="style-label" class:is-here=is_here title=move || help.map(|h| ui.tx(h, &[])).unwrap_or_default()>
            <span class="style-label-text">{ui.t(label)}</span>
        </span>
    };
    let control = match s {
        Setting::Trigger | Setting::Split | Setting::Dir => {
            let options: &'static [(&'static str, &'static str)] = match s {
                Setting::Trigger => &[("view", "anim-trigger-view"), ("load", "anim-trigger-load")],
                Setting::Split => &[
                    ("chars", "anim-split-chars"),
                    ("words", "anim-split-words"),
                    ("lines", "anim-split-lines"),
                ],
                _ => &[("left", "anim-dir-left"), ("right", "anim-dir-right")],
            };
            view! {
                <div class="seg" role="radiogroup">
                    {options.iter().map(|(v, k)| {
                        let v: &'static str = v;
                        let shown = move || {
                            let a = current();
                            a.get(key).map(str::to_string).or_else(|| a.fallback(f, key)).unwrap_or_default()
                        };
                        let on = move || is_here() && shown() == v;
                        let implied = move || !is_here() && shown() == v;
                        view! {
                            <button role="radio" class:is-on=on class:is-implied=implied aria-checked=move || (shown() == v).to_string()
                                title=move || if key == "trigger" { ui.tx(&format!("{k}-help"), &[]) } else { String::new() }
                                on:click=move |_| update(&|a| {
                                    a.set(f, key, Some(v.to_string()));
                                    // Découpe changée : décalage par défaut de la nouvelle découpe.
                                    if key == "split" {
                                        a.set(f, "stagger", None);
                                    }
                                })><span>{ui.t(k)}</span></button>
                        }
                    }).collect_view()}
                </div>
            }
            .into_any()
        }
        Setting::Ease => view! {
            <div class="select">
                <select class="input" on:change=move |e| { let v = event_target_value(&e); update(&|a| a.set(f, key, Some(v.clone()))) }>
                    {["smooth", "bounce", "ease", "linear"].into_iter().map(|v| {
                        let selected = move || {
                            let a = current();
                            a.get(key).map(str::to_string).or_else(|| a.fallback(f, key)).as_deref() == Some(v)
                        };
                        view! { <option value=v selected=selected>{move || ui.tx(&format!("anim-ease-{v}"), &[])}</option> }
                    }).collect_view()}
                </select>
            </div>
        }
        .into_any(),
        Setting::Mirror => view! {
            <label class="switch">
                <input type="checkbox"
                    prop:checked=move || { let a = current(); a.get(key).map(str::to_string).or_else(|| a.fallback(f, key)).as_deref() == Some("on") }
                    on:change=move |e| { let on = event_target_checked(&e); update(&|a| a.set(f, key, Some(if on { "on" } else { "off" }.into()))) }/>
                <span class="switch-track"></span>
            </label>
        }
        .into_any(),
        Setting::Pause | Setting::Replay => view! {
            <label class="switch">
                <input type="checkbox" prop:checked=move || current().has(key)
                    on:change=move |e| { let on = event_target_checked(&e); update(&|a| a.flag(key, on)) }/>
                <span class="switch-track"></span>
            </label>
        }
        .into_any(),
        Setting::Dur | Setting::Delay | Setting::Dist | Setting::Stagger | Setting::Amount => {
            let unit = match s {
                Setting::Dist => "px",
                Setting::Amount => "%",
                _ => "s",
            };
            let shown = move || anim.with(|a| a.as_ref().and_then(|a| a.get(key).map(str::to_string)).unwrap_or_default());
            let placeholder = move || current().fallback(f, key).unwrap_or_default();
            let commit = move |v: String| {
                let v = v.trim().replace(',', ".");
                let v = v.trim_end_matches(unit).trim().to_string();
                if !v.is_empty() && v.parse::<f64>().map_or(true, |n| !n.is_finite() || n < 0.0) {
                    return;
                }
                update(&|a| a.set(f, key, (!v.is_empty()).then(|| v.clone())));
            };
            view! {
                <div class="nbox has-units" class:is-here=is_here>
                    <input class="nbox-input" type="text" inputmode="decimal" spellcheck="false"
                        prop:value=shown placeholder=placeholder aria-label=ui.t(label)
                        on:change=move |e| commit(event_target_value(&e))
                        on:keydown=move |e: leptos::ev::KeyboardEvent| if e.key() == "Enter" { commit(event_target_value(&e)) }/>
                    <span class="nbox-suffix" aria-hidden="true">{unit}</span>
                </div>
            }
            .into_any()
        }
    };
    view! {
        <div class="style-row" data-setting=key>
            {label_view}
            <div class="style-control">{control}</div>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeur_lue_et_ecrite() {
        let mut a = Anim::parse("up dur=0.8 replay");
        assert_eq!(a.preset, "up");
        assert_eq!(a.get("dur"), Some("0.8"));
        assert!(a.has("replay"));
        // Valeur par défaut : retirée.
        a.set(Family::Appear, "dur", Some("0.6".into()));
        a.set(Family::Appear, "delay", Some("0.2".into()));
        assert_eq!(a.write(), "up replay delay=0.2");
        a.flag("replay", false);
        assert_eq!(a.write(), "up delay=0.2");
        // Changement d'animation : réglages qui ne s'appliquent plus retirés.
        let mut b = Anim::parse("up dist=48 delay=0.2");
        b = b.with_preset(Family::Appear, "fade");
        assert_eq!(b.write(), "fade delay=0.2");
        // Boucle : aller-retour par défaut selon l'animation.
        let mut c = Anim::parse("pulse");
        c.set(Family::Loop, "mirror", Some("on".into()));
        assert_eq!(c.write(), "pulse");
        c = c.with_preset(Family::Loop, "spin");
        c.set(Family::Loop, "mirror", Some("on".into()));
        assert_eq!(c.write(), "spin mirror=on");
        // Texte : décalage par défaut selon la découpe.
        let t = Anim::parse("fade split=chars");
        assert_eq!(t.fallback(Family::Text, "stagger").as_deref(), Some("0.03"));
    }

    #[test]
    fn animations_et_libelles() {
        for f in FAMILIES {
            assert!(!f.presets().is_empty());
            for p in f.presets() {
                assert!(!preset_key(f, p).is_empty());
                for s in f.settings(p) {
                    let k = setting_key(s);
                    if !matches!(s, Setting::Replay | Setting::Pause) {
                        assert!(
                            Anim::parse(p).fallback(f, k).is_some(),
                            "{} {p} {k}",
                            f.id()
                        );
                    }
                }
            }
        }
    }
}
