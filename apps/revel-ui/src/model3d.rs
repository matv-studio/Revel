//! Réglages des modèles 3D et des scènes Marmoset (D-160) : fichier, caméra (réglée au
//! glisser dans le canevas ou chiffrée), rendu, animation, boutons visibles.

use crate::Ui;
use crate::editor::Ed;
use leptos::prelude::*;
use revel_edit::{Command, Target};
use revel_model::{Attr, Node};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Animations et caméras du fichier, relevées par le canevas une fois le modèle chargé.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Info {
    pub animations: Vec<String>,
    pub cameras: Vec<String>,
}

// Signaux propres au module (`ArcRwSignal` : rattachés à aucun propriétaire, jamais libérés
// quand les réglages se redessinent).
fn infos() -> ArcRwSignal<HashMap<String, Info>> {
    static S: OnceLock<ArcRwSignal<HashMap<String, Info>>> = OnceLock::new();
    S.get_or_init(|| ArcRwSignal::new(HashMap::new())).clone()
}

/// Modèle dont la caméra se règle au glisser dans le canevas (identifiant).
fn orbiting() -> ArcRwSignal<Option<String>> {
    static S: OnceLock<ArcRwSignal<Option<String>>> = OnceLock::new();
    S.get_or_init(|| ArcRwSignal::new(None)).clone()
}

/// Message du canevas : modèle chargé.
pub fn on_info(id: String, animations: Vec<String>, cameras: Vec<String>) {
    let info = Info {
        animations,
        cameras,
    };
    if infos().with_untracked(|m| m.get(&id) != Some(&info)) {
        infos().update(|m| {
            m.insert(id, info);
        });
    }
}

/// Vue actuelle d'un modèle (canevas), en réglages de l'élément.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct View {
    pub yaw: f64,
    pub pitch: f64,
    pub zoom: f64,
    #[serde(default)]
    pub target: String,
}

/// Message du canevas : vue actuelle d'un modèle, écrite dans ses réglages (une étape).
pub fn on_view(ui: Ui, ed: Ed, id: String, view: View) {
    let Some(at) = ed
        .doc
        .with_untracked(|d| crate::editor::target_of(d.as_ref()?, &id))
    else {
        return;
    };
    let present = present_attrs(ed, &at);
    let mut cmds = Vec::new();
    let mut put = |name: &str, v: Option<String>| {
        if let Some(c) = attr_command(&at, &present, name, v) {
            cmds.push(c);
        }
    };
    put("data-model-yaw", (view.yaw != 0.0).then(|| fmt(view.yaw)));
    put("data-model-pitch", Some(fmt(view.pitch)));
    put(
        "data-model-zoom",
        (view.zoom != 100.0).then(|| fmt(view.zoom)),
    );
    put(
        "data-model-target",
        (!view.target.is_empty()).then_some(view.target),
    );
    ed.apply(ui, Command::Batch { commands: cmds }, false);
}

/// Attributs portés par l'élément (une commande qui retire un attribut absent est refusée).
fn present_attrs(ed: Ed, at: &Target) -> Vec<String> {
    ed.doc.with_untracked(|d| {
        d.as_ref()
            .and_then(|d| revel_edit::address::node(d, at).ok())
            .map(|n| match n {
                Node::Element { attrs, .. } => attrs.iter().map(|a| a.name.clone()).collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default()
    })
}

/// Pose ou retire un attribut ; `None` s'il n'y a rien à faire (retrait d'un attribut absent).
fn attr_command(at: &Target, present: &[String], name: &str, v: Option<String>) -> Option<Command> {
    match v {
        Some(v) => Some(Command::SetAttr {
            at: at.clone(),
            name: name.into(),
            value: Some(v),
        }),
        None => present
            .iter()
            .any(|p| p == name)
            .then(|| Command::RemoveAttr {
                at: at.clone(),
                name: name.into(),
            }),
    }
}

fn fmt(v: f64) -> String {
    revel_style::values::fmt_num((v * 100.0).round() / 100.0)
}

fn value(attrs: &[Attr], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.clone().unwrap_or_default())
}

fn has(attrs: &[Attr], name: &str) -> bool {
    attrs.iter().any(|a| a.name == name)
}

/// Écrit un réglage (`None` : retiré).
fn setter(ui: Ui, ed: Ed, at: StoredValue<Target>) -> impl Fn(&str, Option<String>) + Copy {
    move |name: &str, v: Option<String>| {
        let at = at.get_value();
        let present = present_attrs(ed, &at);
        if let Some(cmd) = attr_command(&at, &present, name, v) {
            ed.apply(ui, cmd, false);
        }
    }
}

/// Aperçu dans le canevas pendant un glisser (sans étape d'annulation).
fn preview(ed: Ed, id: &str, name: &str, v: &str) {
    ed.post(serde_json::json!({
        "type": "attrs",
        "changes": [{ "id": id, "set": { name: v } }],
    }));
}

/// Réglage chiffré : glissière et champ, aperçu en direct, valeur par défaut retirée.
#[allow(clippy::too_many_arguments)]
fn slider(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    attrs: &[Attr],
    name: &'static str,
    key: &'static str,
    (min, max, step, dflt): (f64, f64, f64, f64),
    suffix: &'static str,
    keep_default: bool,
) -> AnyView {
    let set = setter(ui, ed, at);
    let cur = value(attrs, name)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(dflt);
    let shown = RwSignal::new(cur);
    let pct = move |v: f64| ((v - min) / (max - min) * 100.0).clamp(0.0, 100.0);
    let commit = move |v: f64| {
        let v = v.clamp(min, max);
        shown.set(v);
        set(name, (keep_default || v != dflt).then(|| fmt(v)));
    };
    let is_here = has(attrs, name);
    view! {
        <div class="field" data-model-field=name>
            <span class="label" title=ui.tx(&format!("{key}-help"), &[])>{ui.t(key)}</span>
            <div class="ctl-line">
                <input type="range" class="range" class:is-here=is_here
                    min=min.to_string() max=max.to_string() step=step.to_string()
                    prop:value=move || shown.get().to_string()
                    style=move || format!("--fill: {}%", pct(shown.get()))
                    aria-label=ui.t(key)
                    on:input=move |e| {
                        if let Ok(v) = event_target_value(&e).parse::<f64>() {
                            shown.set(v);
                            preview(ed, &at.get_value().node, name, &fmt(v));
                        }
                    }
                    on:change=move |e| { if let Ok(v) = event_target_value(&e).parse::<f64>() { commit(v) } }/>
                <div class="nbox" class:is-here=is_here>
                    <input class="nbox-input" inputmode="decimal" aria-label=ui.t(key)
                        prop:value=move || fmt(shown.get())
                        on:change=move |e| {
                            if let Ok(v) = event_target_value(&e).replace(',', ".").trim().parse::<f64>() { commit(v) }
                        }/>
                    {(!suffix.is_empty()).then(|| view! { <span class="nbox-suffix">{suffix}</span> })}
                </div>
            </div>
        </div>
    }
    .into_any()
}

/// Interrupteur (attribut présent ou absent).
fn toggle(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    attrs: &[Attr],
    name: &'static str,
    key: &'static str,
    inverted: Option<&'static str>,
) -> AnyView {
    let set = setter(ui, ed, at);
    // `inverted` : valeur posée quand l'interrupteur est éteint (`data-model-controls="none"`).
    let on = match inverted {
        Some(v) => value(attrs, name).as_deref() != Some(v),
        None => has(attrs, name),
    };
    view! {
        <label class="view-line" title=ui.tx(&format!("{key}-help"), &[])>
            <span>{ui.t(key)}</span>
            <span class="switch">
                <input type="checkbox" data-model-toggle=name prop:checked=on on:change=move |e| {
                    let c = event_target_checked(&e);
                    match inverted {
                        Some(v) => set(name, (!c).then(|| v.to_string())),
                        None => set(name, c.then(String::new)),
                    }
                }/>
                <span class="switch-track"></span>
            </span>
        </label>
    }
    .into_any()
}

/// Choix d'un fichier des médias : nom, bouton, retrait.
#[allow(clippy::too_many_arguments)]
fn file_field(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    attrs: &[Attr],
    name: &'static str,
    key: &'static str,
    kind: Option<revel_protocol::MediaKind>,
    removable: bool,
) -> AnyView {
    let set = setter(ui, ed, at);
    let src = value(attrs, name).unwrap_or_default();
    let file = src.rsplit('/').next().unwrap_or_default().to_string();
    let pick = move || {
        // Fichier d'un autre format (`.mview` dans un modèle GLB, ou l'inverse) : l'élément passe
        // à ce format, avec le fichier.
        let on = move |u: String| {
            let cur = match name {
                "data-model" => Some("model"),
                "data-mview" => Some("mview"),
                _ => None,
            };
            match (cur, revel_edit::make::model_kind_of_file(&u)) {
                (Some(cur), Some(k)) if k != cur => switch_kind(ui, ed, at, k, Some(u)),
                _ => set(name, Some(u)),
            }
        };
        match kind {
            Some(k) => crate::library::pick_kind(ui, k, on),
            None => crate::library::pick_image(ui, crate::library::media_shelf(ed), on),
        }
    };
    let empty = src.is_empty();
    view! {
        <div class="field">
            <span class="label" title=ui.tx(&format!("{key}-help"), &[])>{ui.t(key)}</span>
            <span class="field-line">
                <input class="input input--mono" readonly=true prop:value=file data-model-file=name
                    placeholder=ui.t("model-none") title=src on:click=move |_| pick()/>
                <button type="button" class="icon-btn icon-btn--small" title=ui.t("model-choose")
                    aria-label=ui.t("model-choose") on:click=move |_| pick()>{if kind.is_some() { crate::icons::cube() } else { crate::icons::image() }}</button>
                {(removable && !empty).then(|| view! {
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("model-remove-file")
                        aria-label=ui.t("model-remove-file") on:click=move |_| set(name, None)>{crate::icons::close()}</button>
                })}
            </span>
        </div>
    }
    .into_any()
}

/// Change le format de l'élément 3D (GLB / glTF ou Marmoset), en une étape annulable.
fn switch_kind(ui: Ui, ed: Ed, at: StoredValue<Target>, to: &'static str, file: Option<String>) {
    let at = at.get_value();
    let text = ui.tx("insert-text-model", &[]);
    let c = ed.doc.with_untracked(|d| {
        d.as_ref().and_then(|d| {
            revel_edit::make::model_kind_command(d, &at, to, file.as_deref(), &text)
                .ok()
                .flatten()
        })
    });
    if let Some(c) = c {
        ed.apply_keep(ui, c)
    }
}

/// Choix du format : un seul élément « Modèle 3D », lu par le lecteur de Revel (GLB, glTF) ou
/// par celui de Marmoset (`.mview`).
fn kind_switch(ui: Ui, ed: Ed, at: StoredValue<Target>, cur: &'static str) -> AnyView {
    view! {
        <div class="field">
            <span class="label" title=ui.t("model-kind-help")>{ui.t("model-kind")}</span>
            <div class="seg model-kind" role="radiogroup" aria-label=ui.t("model-kind")>
                {[("model", "model-kind-gltf", "model-kind-gltf-help"), ("mview", "model-kind-mview", "model-kind-mview-help")]
                    .into_iter()
                    .map(|(v, k, h)| {
                        let on = cur == v;
                        view! {
                            <button type="button" role="radio" data-model-kind=v class:is-on=on
                                aria-checked=on.to_string() title=ui.t(h)
                                on:click=move |_| if !on { switch_kind(ui, ed, at, v, None) }>{ui.t(k)}</button>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
    .into_any()
}

/// Menu de choix (`""` : valeur retirée).
fn select(
    ui: Ui,
    ed: Ed,
    at: StoredValue<Target>,
    name: &'static str,
    key: &'static str,
    cur: String,
    options: Vec<(String, String)>,
) -> AnyView {
    let set = setter(ui, ed, at);
    view! {
        <label class="field">
            <span class="label" title=ui.tx(&format!("{key}-help"), &[])>{ui.t(key)}</span>
            <div class="select">
                <select class="input" data-model-select=name on:change=move |e| {
                    let v = event_target_value(&e);
                    set(name, (!v.is_empty()).then_some(v));
                }>
                    {options.into_iter().map(|(v, label)| {
                        let sel = v == cur;
                        view! { <option value=v selected=sel>{label}</option> }
                    }).collect_view()}
                </select>
            </div>
        </label>
    }
    .into_any()
}

/// Réglages d'un modèle 3D (`data-model`).
pub fn settings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    if !has(attrs, "data-model") {
        return None;
    }
    let id = at.node.clone();
    let owner = at.clone();
    let at = StoredValue::new(at.clone());
    let set = setter(ui, ed, at);
    let info = {
        let id = id.clone();
        Memo::new(move |_| infos().with(|m| m.get(&id).cloned().unwrap_or_default()))
    };
    // Glisser dans le canevas : la caméra tourne ; « Prendre la vue » l'écrit.
    let orbit_on = {
        let id = id.clone();
        Signal::derive(move || orbiting().get().as_deref() == Some(id.as_str()))
    };
    let set_orbit = {
        let id = id.clone();
        move |on: bool| {
            orbiting().set(on.then(|| id.clone()));
            ed.post(serde_json::json!({ "type": "model-orbit", "id": id, "on": on }));
        }
    };
    {
        let id = id.clone();
        on_cleanup(move || {
            if orbiting().get_untracked().as_deref() == Some(id.as_str()) {
                orbiting().set(None);
                ed.post(serde_json::json!({ "type": "model-orbit", "id": id, "on": false }));
            }
        });
    }
    let take = {
        let id = id.clone();
        move || ed.post(serde_json::json!({ "type": "model-view", "id": id }))
    };
    let reset_camera = move || {
        let at = at.get_value();
        let present = present_attrs(ed, &at);
        let commands = ["data-model-yaw", "data-model-zoom", "data-model-target"]
            .into_iter()
            .filter_map(|n| attr_command(&at, &present, n, None))
            .chain(attr_command(
                &at,
                &present,
                "data-model-pitch",
                Some("15".into()),
            ))
            .collect();
        ed.apply(ui, Command::Batch { commands }, false);
    };
    let camera = value(attrs, "data-model-camera").unwrap_or_default();
    let fixed = !camera.is_empty();
    let env = value(attrs, "data-model-env").unwrap_or_default();
    let env_kind = match env.as_str() {
        "" | "studio" => "studio",
        "none" => "none",
        _ => "file",
    };
    let anim = value(attrs, "data-model-animation").unwrap_or_default();
    let view_cur = value(attrs, "data-model-view").unwrap_or_default();
    let views: Vec<(String, String)> = revel_edit::make::MODEL_VIEWS
        .iter()
        .map(|v| {
            (
                if *v == "render" {
                    String::new()
                } else {
                    v.to_string()
                },
                ui.tx(&format!("model-view-{v}"), &[]),
            )
        })
        .collect();
    let buttons: Vec<(&'static str, &'static str)> = vec![
        ("reset", "model-button-reset"),
        ("fullscreen", "model-button-fullscreen"),
        ("rotate", "model-button-rotate"),
        ("play", "model-button-play"),
        ("views", "model-button-views"),
    ];
    let button_rows = buttons
        .into_iter()
        .map(|(cmd, key)| {
            let on = revel_edit::make::model_has_button(node, cmd);
            let owner = owner.clone();
            view! {
                <label class="view-line">
                    <span>{ui.t(key)}</span>
                    <span class="switch">
                        <input type="checkbox" data-model-button=cmd prop:checked=on on:change=move |e| {
                            let want = event_target_checked(&e);
                            let labels: Vec<String> = if cmd == "views" {
                                revel_edit::make::MODEL_VIEWS.iter().map(|v| ui.tx(&format!("model-view-{v}"), &[])).collect()
                            } else {
                                vec![ui.tx(key, &[])]
                            };
                            let c = ed.doc.with_untracked(|d| {
                                d.as_ref().and_then(|d| revel_edit::make::model_button_command(d, &owner, cmd, &labels, want).ok().flatten())
                            });
                            if let Some(c) = c { ed.apply_keep(ui, c) }
                        }/>
                        <span class="switch-track"></span>
                    </span>
                </label>
            }
        })
        .collect_view();
    let a = attrs.clone();
    Some(
        view! {
            <div class="insp-group" data-model-settings="">
                <p class="panel-title">{ui.t("model-title")}</p>
                {kind_switch(ui, ed, at, "model")}
                {file_field(ui, ed, at, attrs, "data-model", "model-file", Some(revel_protocol::MediaKind::Model), false)}
                {file_field(ui, ed, at, attrs, "data-model-poster", "model-poster", None, true)}
            </div>
            <div class="insp-group">
                <p class="panel-title">{ui.t("model-camera")}</p>
                {move || {
                    let cams = info.get().cameras;
                    (!cams.is_empty() || fixed).then(|| {
                        let mut opts = vec![(String::new(), ui.tx("model-camera-set", &[]))];
                        opts.extend(cams.into_iter().map(|c| (c.clone(), c)));
                        select(ui, ed, at, "data-model-camera", "model-camera-file", camera.clone(), opts)
                    })
                }}
                {(!fixed).then(|| view! {
                    <div class="ctl-line model-orbit">
                        <button type="button" class="btn btn--small" data-action="model-orbit"
                            class:is-on=move || orbit_on.get() aria-pressed=move || orbit_on.get().to_string()
                            title=ui.t("model-orbit-help") on:click=move |_| set_orbit(!orbit_on.get_untracked())>
                            {crate::icons::cube()}<span>{ui.t("model-orbit")}</span>
                        </button>
                        <button type="button" class="btn btn--small" data-action="model-take"
                            title=ui.t("model-take-help") on:click=move |_| take()>
                            <span>{ui.t("model-take")}</span>
                        </button>
                        <button type="button" class="icon-btn icon-btn--small" data-action="model-reset"
                            title=ui.t("model-reset-camera") aria-label=ui.t("model-reset-camera")
                            on:click=move |_| reset_camera()>{crate::icons::undo()}</button>
                    </div>
                })}
                {(!fixed).then(|| view! {
                    {slider(ui, ed, at, &a, "data-model-yaw", "model-yaw", (-180.0, 180.0, 1.0, 0.0), "°", false)}
                    {slider(ui, ed, at, &a, "data-model-pitch", "model-pitch", (-89.0, 89.0, 1.0, 15.0), "°", true)}
                    {slider(ui, ed, at, &a, "data-model-zoom", "model-zoom", (25.0, 400.0, 1.0, 100.0), "%", false)}
                    {slider(ui, ed, at, &a, "data-model-fov", "model-fov", (10.0, 90.0, 1.0, 30.0), "°", false)}
                })}
                {toggle(ui, ed, at, attrs, "data-model-controls", "model-interact", Some("none"))}
                {toggle(ui, ed, at, attrs, "data-model-pan", "model-pan", None)}
                {slider(ui, ed, at, attrs, "data-model-rotate", "model-rotate", (-60.0, 60.0, 1.0, 0.0), "°/s", false)}
            </div>
            <div class="insp-group">
                <p class="panel-title">{ui.t("model-render")}</p>
                <label class="field">
                    <span class="label" title=ui.t("model-env-help")>{ui.t("model-env")}</span>
                    <div class="select">
                        <select class="input" data-model-select="data-model-env" on:change=move |e| {
                            match event_target_value(&e).as_str() {
                                "studio" => set("data-model-env", None),
                                "none" => set("data-model-env", Some("none".into())),
                                _ => crate::library::pick_kind(ui, revel_protocol::MediaKind::Other, move |u| set("data-model-env", Some(u))),
                            }
                        }>
                            <option value="studio" selected=env_kind == "studio">{ui.t("model-env-studio")}</option>
                            <option value="none" selected=env_kind == "none">{ui.t("model-env-none")}</option>
                            <option value="file" selected=env_kind == "file">{if env_kind == "file" { env.rsplit('/').next().unwrap_or_default().to_string() } else { ui.tx("model-env-file", &[]) }}</option>
                        </select>
                    </div>
                </label>
                {slider(ui, ed, at, attrs, "data-model-exposure", "model-exposure", (0.1, 3.0, 0.05, 1.0), "", false)}
                {slider(ui, ed, at, attrs, "data-model-shadow", "model-shadow", (0.0, 1.0, 0.05, 0.0), "", false)}
                {toggle(ui, ed, at, attrs, "data-model-backdrop", "model-backdrop", None)}
                {has(attrs, "data-model-backdrop").then(|| slider(ui, ed, at, &a, "data-model-backdrop", "model-blur", (0.0, 1.0, 0.05, 0.0), "", true))}
                {select(ui, ed, at, "data-model-view", "model-view", view_cur, views)}
            </div>
            {move || {
                let list = info.get().animations;
                (!list.is_empty() || !anim.is_empty()).then(|| {
                    let mut opts = vec![(String::new(), ui.tx("model-animation-none", &[]))];
                    opts.extend(list.into_iter().map(|n| (n.clone(), n)));
                    view! {
                        <div class="insp-group">
                            <p class="panel-title">{ui.t("model-animation")}</p>
                            {select(ui, ed, at, "data-model-animation", "model-animation-clip", anim.clone(), opts)}
                            {toggle(ui, ed, at, &a, "data-model-loop", "model-loop", None)}
                            {slider(ui, ed, at, &a, "data-model-speed", "model-speed", (0.1, 3.0, 0.05, 1.0), "×", false)}
                        </div>
                    }
                })
            }}
            <div class="insp-group">
                <p class="panel-title">{ui.t("model-buttons")}</p>
                {button_rows}
            </div>
        }
        .into_any(),
    )
}

/// Réglages d'un modèle 3D au format Marmoset (`data-mview`).
pub fn mview_settings(ui: Ui, ed: Ed, at: &Target, node: &Node) -> Option<AnyView> {
    let Node::Element { attrs, .. } = node else {
        return None;
    };
    if !has(attrs, "data-mview") {
        return None;
    }
    let at = StoredValue::new(at.clone());
    Some(
        view! {
            <div class="insp-group" data-mview-settings="">
                <p class="panel-title">{ui.t("model-title")}</p>
                {kind_switch(ui, ed, at, "mview")}
                {file_field(ui, ed, at, attrs, "data-mview", "mview-file", Some(revel_protocol::MediaKind::Model), false)}
                {file_field(ui, ed, at, attrs, "data-mview-thumbnail", "mview-thumbnail", None, true)}
                {toggle(ui, ed, at, attrs, "data-mview-autostart", "mview-autostart", None)}
                {toggle(ui, ed, at, attrs, "data-mview-transparent", "mview-transparent", None)}
                {toggle(ui, ed, at, attrs, "data-mview-noui", "mview-noui", None)}
            </div>
        }
        .into_any(),
    )
}
