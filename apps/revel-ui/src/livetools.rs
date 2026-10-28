//! Réglages des outils vivants (D-158) : horloge (`data-revel-clock`) et toile de pixels
//! (`data-revel-pixels`). Leurs options sont des mots dans la valeur de l'attribut, réglés ici.

use crate::Ui;
use crate::editor::Ed;
use leptos::prelude::*;
use revel_edit::{Command, Target};
use revel_model::Attr;

/// Options d'un outil : mots seuls (`seconds`) et réglages (`size=7`), dans l'ordre écrit.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Opts(Vec<String>);

impl Opts {
    pub(crate) fn parse(v: &str) -> Self {
        Opts(v.split_whitespace().map(str::to_string).collect())
    }
    pub(crate) fn has(&self, w: &str) -> bool {
        self.0.iter().any(|x| x == w)
    }
    pub(crate) fn get(&self, k: &str) -> Option<String> {
        let pre = format!("{k}=");
        self.0
            .iter()
            .find_map(|x| x.strip_prefix(&pre).map(str::to_string))
    }
    /// Mot seul ajouté ou retiré.
    pub(crate) fn flag(mut self, w: &str, on: bool) -> Self {
        self.0.retain(|x| x != w);
        if on {
            self.0.push(w.to_string());
        }
        self
    }
    /// Un seul mot parmi `group` (aucun : `None`).
    pub(crate) fn one_of(mut self, group: &[&str], w: Option<&str>) -> Self {
        self.0.retain(|x| !group.contains(&x.as_str()));
        if let Some(w) = w {
            self.0.insert(0, w.to_string());
        }
        self
    }
    /// Réglage écrit (sans espace) ou retiré.
    pub(crate) fn set(mut self, k: &str, v: Option<String>) -> Self {
        let pre = format!("{k}=");
        self.0.retain(|x| !x.starts_with(&pre));
        if let Some(v) = v {
            let v: String = v.chars().filter(|c| !c.is_whitespace()).collect();
            if !v.is_empty() {
                self.0.push(format!("{k}={v}"));
            }
        }
        self
    }
    pub(crate) fn text(&self) -> String {
        self.0.join(" ")
    }
}

fn value(attrs: &[Attr], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.name == name)
        .map(|a| a.value.clone().unwrap_or_default())
}

fn writer(ui: Ui, ed: Ed, at: Target, name: &'static str) -> impl Fn(Opts) + Copy + 'static {
    let at = StoredValue::new(at);
    move |o: Opts| {
        ed.apply(
            ui,
            Command::SetAttr {
                at: at.get_value(),
                name: name.into(),
                value: Some(o.text()),
            },
            false,
        )
    }
}

/// Interrupteur d'une ligne de réglages.
fn switch_line(
    ui: Ui,
    key: &'static str,
    help: &'static str,
    on: bool,
    toggle: impl Fn(bool) + 'static,
) -> impl IntoView {
    view! {
        <label class="view-line" data-opt=key>
            <span title=move || if help.is_empty() { String::new() } else { ui.tx(help, &[]) }>{ui.t(key)}</span>
            <span class="switch">
                <input type="checkbox" prop:checked=on on:change=move |e| toggle(event_target_checked(&e))/>
                <span class="switch-track"></span>
            </span>
        </label>
    }
}

/// Horloge : heure, date ou les deux ; secondes, 12 heures ; jour de la semaine, année, mois.
pub fn clock_settings(ui: Ui, ed: Ed, at: &Target, attrs: &[Attr]) -> Option<AnyView> {
    let o = Opts::parse(&value(attrs, "data-revel-clock")?);
    let write = writer(ui, ed, at.clone(), "data-revel-clock");
    let kind = if o.has("date") {
        "date"
    } else if o.has("both") {
        "both"
    } else {
        "time"
    };
    let time = kind != "date";
    let date = kind != "time";
    let month = o.get("month").unwrap_or_else(|| "long".into());
    let kinds = ["time", "date", "both"];
    let (o1, o2, o3, o4, o5, o6) = (
        o.clone(),
        o.clone(),
        o.clone(),
        o.clone(),
        o.clone(),
        o.clone(),
    );
    Some(
        view! {
            <div class="insp-group" data-role="clock">
                <p class="panel-title" title=ui.t("insert-help-clock")>{ui.t("insert-clock")}</p>
                <div class="seg seg--small" role="radiogroup" aria-label=ui.t("clock-show")>
                    {[("time", "clock-time"), ("date", "clock-date"), ("both", "clock-both")].into_iter().map(|(v, k)| {
                        let on = v == kind;
                        let o = o1.clone();
                        view! {
                            <button role="radio" data-kind=v class:is-on=on aria-checked=on.to_string()
                                on:click=move |_| if !on { write(o.clone().one_of(&kinds, (v != "time").then_some(v))) }>
                                <span>{ui.t(k)}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
                {time.then(|| view! {
                    {switch_line(ui, "clock-seconds", "", o.has("seconds"), move |on| write(o2.clone().flag("seconds", on)))}
                    {switch_line(ui, "clock-12h", "", o.has("12h"), move |on| write(o3.clone().flag("12h", on)))}
                })}
                {date.then(|| view! {
                    {switch_line(ui, "clock-weekday", "", o.has("weekday"), move |on| write(o4.clone().flag("weekday", on)))}
                    {switch_line(ui, "clock-year", "", o.has("year"), move |on| write(o5.clone().flag("year", on)))}
                    <label class="field">
                        <span class="label">{ui.t("clock-month")}</span>
                        <div class="select">
                            <select class="input" data-opt="month" on:change=move |e| {
                                let v = event_target_value(&e);
                                write(o6.clone().set("month", (v != "long").then_some(v)))
                            }>
                                {[("long", "clock-month-long"), ("short", "clock-month-short"), ("numeric", "clock-month-numeric"), ("none", "clock-month-none")]
                                    .into_iter().map(|(v, k)| view! { <option value=v prop:selected=month == v>{ui.t(k)}</option> }).collect_view()}
                            </select>
                        </div>
                    </label>
                })}
            </div>
        }
        .into_any(),
    )
}

/// Toile de pixels : dessin ou traînée, taille et couleur des pixels, au survol ou en appuyant,
/// durée de la traînée.
pub fn pixels_settings(ui: Ui, ed: Ed, at: &Target, attrs: &[Attr]) -> Option<AnyView> {
    let o = Opts::parse(&value(attrs, "data-revel-pixels")?);
    let write = writer(ui, ed, at.clone(), "data-revel-pixels");
    let draw = o.has("draw");
    let size = o.get("size").unwrap_or_else(|| "20".into());
    let fade = o.get("fade").unwrap_or_else(|| "1".into());
    let color = o.get("color");
    let sx = crate::styles::Sx::new(ui, ed);
    let (o1, o2, o3, o4, o5) = (o.clone(), o.clone(), o.clone(), o.clone(), o.clone());
    let num = |t: String, max: f64| {
        t.trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|v| *v > 0.0)
            .map(|v| revel_style::values::fmt_num(v.min(max)))
    };
    let c1 = color.clone();
    Some(
        view! {
            <div class="insp-group" data-role="pixels">
                <p class="panel-title" title=ui.t("insert-help-pixels")>{ui.t("insert-pixels")}</p>
                <div class="seg seg--small" role="radiogroup" aria-label=ui.t("pixels-mode")>
                    {[("trail", "pixels-trail", "pixels-trail-help"), ("draw", "pixels-draw", "pixels-draw-help")].into_iter().map(|(v, k, h)| {
                        let on = (v == "draw") == draw;
                        let o = o1.clone();
                        view! {
                            <button role="radio" data-mode=v class:is-on=on aria-checked=on.to_string() title=ui.t(h)
                                on:click=move |_| if !on { write(o.clone().one_of(&["draw", "trail"], Some(v))) }>
                                <span>{ui.t(k)}</span>
                            </button>
                        }
                    }).collect_view()}
                </div>
                <div class="style-row" title=ui.t("pixels-size-help")>
                    <span class="style-label"><span class="style-label-text">{ui.t("pixels-size")}</span></span>
                    <div class="style-control">
                        <div class="nbox has-units">
                            <input class="nbox-input" type="text" inputmode="decimal" data-opt="size" prop:value=size
                                on:change=move |e| if let Some(v) = num(event_target_value(&e), 400.0) { write(o2.clone().set("size", Some(v))) }/>
                            <span class="nbox-suffix" aria-hidden="true">"px"</span>
                        </div>
                    </div>
                </div>
                <div class="style-row">
                    <span class="style-label"><span class="style-label-text">{ui.t("pixels-color")}</span></span>
                    <div class="style-control">
                        <crate::styles::ColorInput sx
                            value=Signal::derive(move || c1.clone())
                            hint=Signal::derive(|| "#000000".to_string())
                            resolved=Signal::derive(move || color.clone().unwrap_or_else(|| "#000000".into()))
                            on_preview=Callback::new(|_: String| {})
                            on_commit=Callback::new(move |v: Option<String>| write(o3.clone().set("color", v)))
                            label="pixels-color"/>
                    </div>
                </div>
                {switch_line(ui, "pixels-click", "pixels-click-help", o.has("click"), move |on| write(o4.clone().flag("click", on)))}
                {(!draw).then(|| view! {
                    <div class="style-row" title=ui.t("pixels-fade-help")>
                        <span class="style-label"><span class="style-label-text">{ui.t("pixels-fade")}</span></span>
                        <div class="style-control">
                            <div class="nbox has-units">
                                <input class="nbox-input" type="text" inputmode="decimal" data-opt="fade" prop:value=fade
                                    on:change=move |e| if let Some(v) = num(event_target_value(&e), 30.0) { write(o5.clone().set("fade", (v != "1").then_some(v))) }/>
                                <span class="nbox-suffix" aria-hidden="true">"s"</span>
                            </div>
                        </div>
                    </div>
                })}
            </div>
        }
        .into_any(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options() {
        let o = Opts::parse("draw size=7 color=rgb(0,0,0) click");
        assert!(o.has("draw") && o.has("click"));
        assert_eq!(o.get("size").as_deref(), Some("7"));
        let o = o
            .one_of(&["draw", "trail"], Some("trail"))
            .set("color", Some("rgb(1, 2, 3)".into()))
            .flag("click", false);
        assert_eq!(o.text(), "trail size=7 color=rgb(1,2,3)");
        assert_eq!(
            Opts::parse("date year")
                .one_of(&["time", "date", "both"], None)
                .text(),
            "year"
        );
    }
}
