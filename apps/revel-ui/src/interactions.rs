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

/// Moment d'animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Appear,
    Text,
    Hover,
    Press,
    Loop,
    Scroll,
    /// Devenu actif (onglet, filtre, interrupteur).
    Active,
}

pub const FAMILIES: [Family; 7] = [
    Family::Appear,
    Family::Text,
    Family::Hover,
    Family::Press,
    Family::Loop,
    Family::Scroll,
    Family::Active,
];

/// Réglage proposé.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Setting {
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
    /// Défilement : vitesse (px/s).
    Speed,
    /// Défilement : bords estompés (px).
    Fade,
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
            Family::Active => "active",
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
            Family::Active => "anim-active",
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
            Family::Active => "anim-active-help",
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
            Family::Active => &["pop", "jump", "shake", "flash", "spin"],
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
                "ticker" => vec![Speed, Dir, Pause, Fade],
                "spin" => vec![Dur, Delay, Mirror],
                "pulse" | "float" | "swing" | "blink" => vec![Dur, Delay, Ease, Amount, Mirror],
                _ => vec![Dur, Delay, Amount],
            },
            Family::Scroll => vec![Amount],
            Family::Active => vec![Dur, Ease, Amount],
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
        (Family::Loop, "ticker", "speed") => Some("60"),
        (Family::Loop, "ticker", "fade") => Some("0"),
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
        (Family::Active, "dur") => Some("0.25"),
        (Family::Active, "ease") => Some("bounce"),
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
        Setting::Speed => "speed",
        Setting::Fade => "fade",
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

/// Panneau « Interactions ».
#[component]
pub fn Interactions() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
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
        <DragSection at=at/>
        <ActionsSection at=at/>
    }
}

const TRIGGERS: &[(&str, &str)] = &[
    ("click", "act-trigger-click"),
    ("hover", "act-trigger-hover"),
    ("scroll", "act-trigger-scroll"),
    ("view", "act-trigger-view"),
    ("load", "act-trigger-load"),
];
const ACTS: &[(&str, &str)] = &[
    ("class", "act-class"),
    ("toggle", "act-toggle"),
    ("show", "act-show"),
    ("hide", "act-hide"),
    ("play", "act-play"),
    ("dismiss", "act-dismiss"),
    ("random", "act-random"),
    ("spin", "act-spin"),
    ("sound", "act-sound"),
    ("music", "act-music"),
];

/// Classes et identifiants du site (éléments visés).
/// Classes des éléments qui contiennent `at`, du plus proche au plus lointain.
fn ancestor_classes(d: &revel_model::Site, at: &revel_edit::address::Target) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = at.clone();
    while let Ok((parent, _)) = revel_edit::address::locate(d, &cur) {
        let Some(id) = parent.node else { break };
        cur = revel_edit::address::Target {
            owner: parent.owner,
            node: id,
        };
        if let Ok(Node::Element { attrs, .. }) = revel_edit::address::node(d, &cur) {
            for c in attrs
                .iter()
                .find(|a| a.name == "class")
                .and_then(|a| a.value.as_deref())
                .unwrap_or("")
                .split_whitespace()
            {
                if revel_edit::actions::target_ok(&format!(".{c}")) && !out.iter().any(|x| x == c) {
                    out.push(c.to_string());
                }
            }
        }
    }
    out
}

fn site_targets(d: &revel_model::Site) -> (Vec<String>, Vec<String>) {
    let mut classes: Vec<String> = d
        .style_rules()
        .filter_map(|r| {
            let s = r.selector.trim();
            let c = s.strip_prefix('.')?;
            c.chars()
                .all(|x| x.is_alphanumeric() || x == '-' || x == '_')
                .then(|| c.to_string())
        })
        .collect();
    classes.sort();
    classes.dedup();
    let mut ids = Vec::new();
    for (_, _, body) in d.owners() {
        revel_model::ids::walk(body, &mut |n| {
            if let Node::Element { attrs, .. } = n
                && let Some(v) = attrs
                    .iter()
                    .find(|a| a.name == "id")
                    .and_then(|a| a.value.clone())
                && revel_edit::actions::target_ok(&format!("#{v}"))
            {
                ids.push(v);
            }
        });
    }
    ids.sort();
    ids.dedup();
    (classes, ids)
}

/// Interactions entre éléments (6.3) : déclencheur, action, élément visé (`data-revel-on`).
#[component]
fn ActionsSection(at: StoredValue<Target>) -> impl IntoView {
    use revel_edit::actions::{Act, Rule, Trigger};
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let value = Memo::new(move |_| {
        let at = at.get_value();
        ed.doc.with(
            |d| match revel_edit::address::node(d.as_ref()?, &at).ok()? {
                Node::Element { attrs, .. } | Node::Use { attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == "data-revel-on")
                    .and_then(|a| a.value.clone()),
                _ => None,
            },
        )
    });
    let rules = Memo::new(move |_| value.get().map(|v| revel_edit::actions::parse(&v)));
    let write = move |list: Vec<Rule>| {
        let at = at.get_value();
        let cmd = if list.is_empty() {
            Command::RemoveAttr {
                at,
                name: "data-revel-on".into(),
            }
        } else {
            Command::SetAttr {
                at,
                name: "data-revel-on".into(),
                value: Some(revel_edit::actions::write(&list)),
            }
        };
        ed.apply(ui, cmd, false);
    };
    let edit = move |i: usize, f: Box<dyn Fn(&mut Rule)>| {
        let mut l = rules.get_untracked().flatten().unwrap_or_default();
        if let Some(r) = l.get_mut(i) {
            f(r);
        }
        write(l);
    };
    let add = move |_| {
        let mut l = rules.get_untracked().flatten().unwrap_or_default();
        l.push(Rule {
            trigger: Trigger::Click,
            act: Act::Class("is-open".into()),
            target: "self".into(),
        });
        write(l);
    };
    let targets = Memo::new(move |_| {
        ed.doc
            .with(|d| d.as_ref().map(site_targets).unwrap_or_default())
    });
    // Classes des éléments qui contiennent celui-ci (cible « … qui le contient »).
    let enclosing = Memo::new(move |_| {
        let at = at.get_value();
        ed.doc.with(|d| {
            d.as_ref()
                .map(|d| ancestor_classes(d, &at))
                .unwrap_or_default()
        })
    });
    view! {
        <section class="anim-family act-family" class:is-on=move || value.with(Option::is_some) data-family="actions">
            <header class="anim-head">
                <span class="anim-title" title=ui.t("act-help")>{ui.t("act-title")}</span>
                <button class="icon-btn icon-btn--small anim-add" title=ui.t("act-add") aria-label=ui.t("act-add") on:click=add>{icons::plus()}</button>
            </header>
            {move || match rules.get() {
                None => ().into_any(),
                // Forme écrite à la main : modifiable en texte.
                Some(None) => view! {
                    <input class="input input--mono" prop:value=value.get().unwrap_or_default() on:change=move |e| {
                        let v = event_target_value(&e);
                        let at = at.get_value();
                        ed.apply(ui, if v.trim().is_empty() {
                            Command::RemoveAttr { at, name: "data-revel-on".into() }
                        } else {
                            Command::SetAttr { at, name: "data-revel-on".into(), value: Some(v) }
                        }, false);
                    }/>
                }.into_any(),
                Some(Some(list)) => list.into_iter().enumerate().map(|(i, r)| {
                    let trig = match r.trigger { Trigger::Click => "click", Trigger::Hover => "hover", Trigger::Scroll(_) => "scroll", Trigger::View => "view", Trigger::Load => "load" };
                    let px = match r.trigger { Trigger::Scroll(px) => Some(px), _ => None };
                    let (act, class) = match &r.act {
                        Act::Class(c) => ("class", Some(c.clone())),
                        Act::Toggle => ("toggle", None),
                        Act::Show => ("show", None),
                        Act::Hide => ("hide", None),
                        Act::Play => ("play", None),
                        Act::Dismiss => ("dismiss", None),
                        Act::Random(_) => ("random", None),
                        Act::Spin(_) => ("spin", None),
                        Act::Sound(_) => ("sound", None),
                        Act::Music(_) => ("music", None),
                    };
                    // Sons : fichiers (un pris au hasard) et volume.
                    let sounds = match &r.act {
                        Act::Sound(a) | Act::Music(a) => Some(a.clone()),
                        _ => None,
                    };
                    // Délai (au hasard) ou angle (rotation) réglables.
                    let amount = match &r.act {
                        Act::Random(ms) => Some(("act-random-delay", *ms)),
                        Act::Spin(deg) => Some(("act-spin-deg", *deg)),
                        _ => None,
                    };
                    let target = r.target.clone();
                    let target2 = target.clone();
                    view! {
                        <div class="shadow-card act-card">
                            <div class="ctl-line">
                                <div class="select">
                                    <select class="input" aria-label=ui.t("act-trigger") title=ui.t("act-trigger") on:change=move |e| {
                                        let v = event_target_value(&e);
                                        edit(i, Box::new(move |r| r.trigger = match v.as_str() {
                                            "hover" => Trigger::Hover,
                                            "scroll" => Trigger::Scroll(80),
                                            "view" => Trigger::View,
                                            "load" => Trigger::Load,
                                            _ => Trigger::Click,
                                        }));
                                    }>
                                        {TRIGGERS.iter().map(|(v, k)| view! { <option value=*v selected=*v == trig>{ui.t(k)}</option> }).collect_view()}
                                    </select>
                                </div>
                                <button class="icon-btn icon-btn--small" title=ui.t("act-remove") aria-label=ui.t("act-remove")
                                    on:click=move |_| { let mut l = rules.get_untracked().flatten().unwrap_or_default(); if i < l.len() { l.remove(i); } write(l); }>
                                    {icons::trash()}
                                </button>
                            </div>
                            {px.map(|px| view! {
                                <label class="field">
                                    <span class="label">{ui.t("act-scroll-px")}</span>
                                    <input class="input" type="number" min="0" step="10" prop:value=px.to_string() on:change=move |e| {
                                        let n = event_target_value(&e).trim().parse::<u32>().unwrap_or(0);
                                        edit(i, Box::new(move |r| r.trigger = Trigger::Scroll(n)));
                                    }/>
                                </label>
                            })}
                            <div class="select">
                                <select class="input" aria-label=ui.t("act-action") title=ui.t("act-action") on:change=move |e| {
                                    let v = event_target_value(&e);
                                    edit(i, Box::new(move |r| r.act = match v.as_str() {
                                        "toggle" => Act::Toggle,
                                        "show" => Act::Show,
                                        "hide" => Act::Hide,
                                        "play" => Act::Play,
                                        "dismiss" => Act::Dismiss,
                                        "random" => Act::Random(0),
                                        "spin" => Act::Spin(360),
                                        "sound" => Act::Sound(revel_edit::actions::Audio { urls: Vec::new(), volume: 100 }),
                                        "music" => Act::Music(revel_edit::actions::Audio { urls: Vec::new(), volume: 100 }),
                                        _ => Act::Class("is-open".into()),
                                    }));
                                }>
                                    {ACTS.iter().map(|(v, k)| view! { <option value=*v selected=*v == act title=ui.tx(&format!("{k}-help"), &[])>{ui.t(k)}</option> }).collect_view()}
                                </select>
                            </div>
                            {amount.map(|(k, n)| view! {
                                <label class="field">
                                    <span class="label">{ui.t(k)}</span>
                                    <input class="input" type="number" min="0" step="10" prop:value=n.to_string() on:change=move |e| {
                                        let n = event_target_value(&e).trim().parse::<u32>().unwrap_or(0);
                                        edit(i, Box::new(move |r| r.act = match r.act {
                                            Act::Random(_) => Act::Random(n),
                                            _ => Act::Spin(n),
                                        }));
                                    }/>
                                </label>
                            })}
                            {sounds.map(|a| audio_settings(ui, ed, i, a, edit))}
                            {class.map(|c| {
                                // Classes du site proposées (états `is-…` d'abord) ; nom refusé : dit pourquoi.
                                let bad = RwSignal::new(false);
                                let mut names: Vec<String> = ed.doc.with_untracked(|d| {
                                    d.as_ref().map(|d| revel_edit::classes::inventory(d).into_iter().map(|c| c.name).collect()).unwrap_or_default()
                                });
                                names.sort_by_key(|n| (!n.starts_with("is-"), n.clone()));
                                let list_id = format!("act-classes-{i}");
                                view! {
                                <label class="field" class:is-invalid=move || bad.get()>
                                    <span class="label">{ui.t("act-class-name")}</span>
                                    <input class="input input--mono" prop:value=c spellcheck="false" list=list_id.clone()
                                        on:input=move |_| bad.set(false)
                                        on:change=move |e| {
                                        let v = event_target_value(&e).trim().trim_start_matches('.').to_string();
                                        if !v.is_empty() && v.chars().all(|x| x.is_alphanumeric() || x == '-' || x == '_') {
                                            bad.set(false);
                                            edit(i, Box::new(move |r| r.act = Act::Class(v.clone())));
                                        } else {
                                            bad.set(true);
                                        }
                                    }/>
                                    <datalist id=list_id>{names.into_iter().map(|n| view! { <option value=n></option> }).collect_view()}</datalist>
                                    {move || bad.get().then(|| view! { <span class="field-error" role="alert">{ui.t("act-class-invalid")}</span> })}
                                </label>
                                }
                            })}
                            <label class="field">
                                <span class="label">{ui.t("act-target")}</span>
                                <div class="select">
                                    <select class="input" on:change=move |e| {
                                        let v = event_target_value(&e);
                                        edit(i, Box::new(move |r| r.target = v.clone()));
                                    }>
                                        {[("self", "act-target-self"), ("parent", "act-target-parent"), ("next", "act-target-next"), ("body", "act-target-body")]
                                            .into_iter().map(|(v, k)| view! { <option value=v selected=v == target>{ui.t(k)}</option> }).collect_view()}
                                        {move || {
                                            let (classes, ids) = targets.get();
                                            let around = enclosing.get();
                                            let cur = target2.clone();
                                            let known = matches!(cur.as_str(), "self" | "parent" | "next" | "body")
                                                || classes.iter().any(|c| format!(".{c}") == cur)
                                                || around.iter().any(|c| format!("closest:.{c}") == cur)
                                                || ids.iter().any(|x| format!("#{x}") == cur);
                                            let extra = (!known).then(|| view! { <option value=cur.clone() selected=true>{cur.clone()}</option> });
                                            view! {
                                                {extra}
                                                {around.into_iter().map(|c| {
                                                    let v = format!("closest:.{c}");
                                                    let on = v == cur;
                                                    view! { <option value=v.clone() selected=on>{ui.tx("act-target-closest", &[("class", c.clone())])}</option> }
                                                }).collect_view()}
                                                {ids.into_iter().map(|x| {
                                                    let v = format!("#{x}");
                                                    let on = v == cur;
                                                    view! { <option value=v.clone() selected=on>{ui.tx("act-target-id", &[("id", x.clone())])}</option> }
                                                }).collect_view()}
                                                {classes.into_iter().map(|c| {
                                                    let v = format!(".{c}");
                                                    let on = v == cur;
                                                    view! { <option value=v.clone() selected=on>{ui.tx("act-target-class", &[("class", c.clone())])}</option> }
                                                }).collect_view()}
                                            }
                                        }}
                                    </select>
                                </div>
                            </label>
                        </div>
                    }
                }).collect_view().into_any(),
            }}
        </section>
    }
}

/// Sons d'une action « Son » ou « Musique » : fichiers (un pris au hasard à chaque fois) et
/// volume.
fn audio_settings(
    ui: Ui,
    ed: Ed,
    i: usize,
    a: revel_edit::actions::Audio,
    edit: impl Fn(usize, Box<dyn Fn(&mut revel_edit::actions::Rule)>) + Copy + Send + Sync + 'static,
) -> AnyView {
    use revel_edit::actions::{Act, Audio};
    let change = move |f: Box<dyn Fn(&mut Audio)>| {
        edit(
            i,
            Box::new(move |r| match &mut r.act {
                Act::Sound(x) | Act::Music(x) => f(x),
                _ => {}
            }),
        )
    };
    let name = |u: &str| {
        u.rsplit('/')
            .next()
            .unwrap_or(u)
            .replace("%20", " ")
            .to_string()
    };
    let vol = a.volume;
    view! {
        <div class="act-sounds">
            {a.urls.iter().enumerate().map(|(k, u)| view! {
                <div class="ctl-line act-sound">
                    <span class="act-sound-name" title=u.clone()>{icons::audio()}<span>{name(u)}</span></span>
                    <button class="icon-btn icon-btn--small" title=ui.t("act-sound-remove") aria-label=ui.t("act-sound-remove")
                        on:click=move |_| change(Box::new(move |x| { if k < x.urls.len() { x.urls.remove(k); } }))>
                        {icons::close()}</button>
                </div>
            }).collect_view()}
            <button class="link-btn" on:click=move |_| {
                crate::library::pick_audio(ui, crate::library::media_shelf(ed), move |url| {
                    change(Box::new(move |x| x.urls.push(url.clone())));
                })
            }>{ui.t("act-sound-add")}</button>
            <label class="field">
                <span class="label">{ui.t("act-sound-volume")}</span>
                <input class="input" type="number" min="0" max="100" step="5" prop:value=vol.to_string()
                    on:change=move |e| {
                        let n = event_target_value(&e).trim().parse::<u32>().unwrap_or(100).min(100);
                        change(Box::new(move |x| x.volume = n));
                    }/>
            </label>
        </div>
    }
    .into_any()
}

/// Réglages de l'élément « Défilement » (animation en boucle `ticker`), en tête de l'onglet
/// Styles : vitesse, sens, arrêt au survol, bords estompés.
/// Glisser à la souris ou au doigt (D-154) : `data-revel-drag` et ses réglages (sens, rester dans
/// le conteneur, élan, collisions entre les éléments à glisser d'un même conteneur).
#[component]
fn DragSection(at: StoredValue<Target>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let value = Memo::new(move |_| {
        let at = at.get_value();
        ed.doc.with(
            |d| match revel_edit::address::node(d.as_ref()?, &at).ok()? {
                Node::Element { attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == "data-revel-drag")
                    .map(|a| a.value.clone().unwrap_or_default()),
                _ => None,
            },
        )
    });
    let has = move |k: &str| {
        value.with(|v| {
            v.as_deref().is_some_and(|v| {
                v.split_whitespace()
                    .any(|o| o == k || o.starts_with(&format!("{k}=")))
            })
        })
    };
    let axis = move || {
        value.with(|v| {
            v.as_deref()
                .and_then(|v| v.split_whitespace().find_map(|o| o.strip_prefix("axis=")))
                .unwrap_or("")
                .to_string()
        })
    };
    let write = move |v: Option<String>| {
        let at = at.get_value();
        let cmd = match v {
            Some(v) => Command::SetAttr {
                at,
                name: "data-revel-drag".into(),
                value: Some(v),
            },
            None => Command::RemoveAttr {
                at,
                name: "data-revel-drag".into(),
            },
        };
        ed.apply(ui, cmd, false);
    };
    // Réglages réécrits dans un ordre fixe.
    let set = move |axis_v: Option<String>, flag: Option<(&'static str, bool)>| {
        let cur = value.get_untracked().unwrap_or_default();
        let mut flags: Vec<&str> = ["bounds", "momentum", "collide"]
            .into_iter()
            .filter(|k| cur.split_whitespace().any(|o| o == *k))
            .collect();
        if let Some((k, on)) = flag {
            flags.retain(|x| *x != k);
            if on {
                flags.push(k);
            }
        }
        let ax = axis_v.unwrap_or_else(|| {
            cur.split_whitespace()
                .find_map(|o| o.strip_prefix("axis="))
                .unwrap_or("")
                .to_string()
        });
        let mut parts: Vec<String> = Vec::new();
        if !ax.is_empty() {
            parts.push(format!("axis={ax}"));
        }
        for k in ["bounds", "momentum", "collide"] {
            if flags.contains(&k) {
                parts.push(k.into());
            }
        }
        write(Some(parts.join(" ")));
    };
    let toggle_row = move |k: &'static str, label: &'static str, help: &'static str| {
        view! {
            <label class="view-line" title=ui.t(help)>
                <span>{ui.t(label)}</span>
                <span class="switch">
                    <input type="checkbox" data-drag=k prop:checked=move || has(k)
                        on:change=move |e| set(None, Some((k, event_target_checked(&e))))/>
                    <span class="switch-track"></span>
                </span>
            </label>
        }
    };
    view! {
        <section class="anim-family drag-section" class:is-on=move || value.with(Option::is_some) data-family="drag">
            <header class="anim-head">
                <span class="anim-title" title=ui.t("drag-help")>{ui.t("drag-title")}</span>
                {move || match value.get() {
                    None => view! {
                        <button class="icon-btn icon-btn--small anim-add" data-drag="add"
                            title=move || ui.tx("anim-add", &[("name", ui.tx("drag-title", &[]))])
                            aria-label=move || ui.tx("anim-add", &[("name", ui.tx("drag-title", &[]))])
                            on:click=move |_| write(Some("momentum".into()))>{icons::plus()}</button>
                    }.into_any(),
                    Some(_) => view! {
                        <button class="icon-btn icon-btn--small anim-remove" title=ui.t("anim-remove") aria-label=ui.t("anim-remove")
                            on:click=move |_| write(None)>{icons::trash()}</button>
                    }.into_any(),
                }}
            </header>
            <Show when=move || value.with(Option::is_some)>
                <div class="seg drag-axis" role="radiogroup" aria-label=ui.t("drag-axis")>
                    {[("", "drag-axis-free"), ("x", "drag-axis-x"), ("y", "drag-axis-y")].into_iter().map(|(v, k)| {
                        let on = move || axis() == v;
                        view! {
                            <button role="radio" data-axis=v class:is-on=on aria-checked=move || on().to_string()
                                on:click=move |_| set(Some(v.to_string()), None)>{ui.t(k)}</button>
                        }
                    }).collect_view()}
                </div>
                {toggle_row("bounds", "drag-bounds", "drag-bounds-help")}
                {toggle_row("momentum", "drag-momentum", "drag-momentum-help")}
                {toggle_row("collide", "drag-collide", "drag-collide-help")}
            </Show>
        </section>
    }
}

#[component]
pub fn TickerSettings(at: Target) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let f = Family::Loop;
    let at = StoredValue::new(at);
    let anim = Memo::new(move |_| {
        let at = at.get_value();
        ed.doc.with(
            |d| match revel_edit::address::node(d.as_ref()?, &at).ok()? {
                Node::Element { attrs, .. } | Node::Use { attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == f.attr())
                    .and_then(|a| a.value.as_deref())
                    .map(Anim::parse)
                    .filter(|a| a.preset == "ticker"),
                _ => None,
            },
        )
    });
    let write = StoredValue::new(move |a: Option<Anim>| {
        let Some(a) = a else { return };
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: f.attr(),
                value: Some(a.write()),
            },
            false,
        );
        let id = at.get_value().node;
        let v = a.write();
        set_timeout(
            move || {
                ed.post(serde_json::json!({ "type": "animate", "id": id, "family": "loop", "value": v }))
            },
            std::time::Duration::from_millis(350),
        );
    });
    view! {
        <div class="insp-group ticker-settings">
            <p class="panel-title">{ui.t("ticker-title")}</p>
            {[Setting::Speed, Setting::Dir, Setting::Pause, Setting::Fade]
                .into_iter()
                .map(|s| setting_row(ui, f, s, anim, write))
                .collect_view()}
            // Éléments qui défilent : ajouter, remplacer, ranger, retirer (D-123).
            <crate::items::ItemsList at=at.get_value()/>
        </div>
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
                        view! {
                            <button class="anim-tile" role="radio" class:is-on=on aria-checked=move || on().to_string()
                                data-preset=p title=move || ui.tx(&format!("anim-d-{}-{p}", f.id()), &[])
                                on:click=move |_| {
                                    let a = anim.get_untracked().unwrap_or_default();
                                    if a.preset != p {
                                        write.with_value(|w| w(Some(a.with_preset(f, p))));
                                    } else {
                                        preview();
                                    }
                                }>
                                // Dessin du mouvement, joué lentement au survol.
                                <span class="anim-stage" inner_html=crate::animicons::icon(f.id(), p)></span>
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
pub(crate) fn setting_row(
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
        Setting::Speed => ("anim-speed", Some("anim-speed-help")),
        Setting::Fade => ("anim-fade", Some("anim-fade-help")),
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
                _ => &[
                    ("left", "anim-dir-left"),
                    ("right", "anim-dir-right"),
                    ("up", "anim-dir-up"),
                    ("down", "anim-dir-down"),
                ],
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
                                title=move || match key {
                                    "trigger" => ui.tx(&format!("{k}-help"), &[]),
                                    "dir" => ui.tx(k, &[]),
                                    _ => String::new(),
                                }
                                aria-label=move || ui.tx(k, &[])
                                on:click=move |_| update(&|a| {
                                    a.set(f, key, Some(v.to_string()));
                                    // Découpe changée : décalage par défaut de la nouvelle découpe.
                                    if key == "split" {
                                        a.set(f, "stagger", None);
                                    }
                                })>{
                                    // Sens : flèches (libellé en info-bulle).
                                    match (key, v) {
                                        ("dir", "left") => icons::arrow_left(),
                                        ("dir", "right") => icons::arrow_right(),
                                        ("dir", "up") => icons::arrow_up(),
                                        ("dir", "down") => icons::arrow_down(),
                                        _ => view! { <span>{ui.t(k)}</span> }.into_any(),
                                    }
                                }</button>
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
        Setting::Dur
        | Setting::Delay
        | Setting::Dist
        | Setting::Stagger
        | Setting::Amount
        | Setting::Speed
        | Setting::Fade => {
            let unit = match s {
                Setting::Dist | Setting::Fade => "px",
                Setting::Speed => "px/s",
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
