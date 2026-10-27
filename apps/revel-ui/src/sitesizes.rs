//! Tailles du site (Paramètres › Général › Tailles) : variables CSS de `:root` qui donnent une
//! taille (texte, espace, largeur), comme les variables de taille de Webflow. Chacune se renomme
//! partout, se règle (valeur fixe, ou taille fluide entre un minimum et un maximum selon la
//! largeur de l'écran), s'ajoute et se retire. Les classes les lisent (`font-size: var(--titre-l)`).

use crate::site::DocSession;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::Command;
use revel_edit::style::{find_rule, style_command};
use revel_model::Site;
use revel_style::values::color::Rgba;

/// Valeur d'une taille : longueur (`1.25rem`), ou fonction de calcul (`clamp(…)`, `min`, `max`,
/// `calc`) sans couleur ni dégradé.
pub fn is_size(v: &str) -> bool {
    let v = v.trim().to_ascii_lowercase();
    if v.is_empty() || Rgba::parse(&v).is_some() || v.contains("gradient") || v.contains('"') {
        return false;
    }
    let unit = |t: &str| {
        let num: String = t
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        !num.is_empty()
            && num.parse::<f64>().is_ok()
            && matches!(
                &t[num.len()..],
                "px" | "rem" | "em" | "vw" | "vh" | "svh" | "dvh" | "%" | "ch" | ""
            )
    };
    ["clamp(", "min(", "max(", "calc("]
        .iter()
        .any(|f| v.starts_with(f))
        || (!v.contains(' ') && unit(&v))
}

/// Tailles du site : (nom sans `--`, valeur).
fn sizes(d: &Site) -> Vec<(String, String)> {
    let Some(root) = find_rule(d, ":root", &None) else {
        return Vec::new();
    };
    root.decls
        .iter()
        .filter_map(|x| Some((x.prop.strip_prefix("--")?.to_string(), x.value.clone())))
        .filter(|(_, v)| is_size(v))
        .collect()
}

/// Nombre d'utilisations d'une variable dans les styles ; utilisée comme taille de texte.
fn uses(d: &Site, name: &str) -> (usize, bool) {
    let needle = format!("var(--{name}");
    let mut n = 0;
    let mut text = false;
    for decl in d.style_rules().flat_map(|r| r.decls.iter()) {
        if decl.value.contains(&needle) {
            n += 1;
            text |= decl.prop == "font-size" || decl.prop == "font";
        }
    }
    (n, text)
}

/// `clamp(min, fluide, max)` découpé au premier niveau.
fn clamp_parts(v: &str) -> Option<[String; 3]> {
    let inner = v.trim().strip_prefix("clamp(")?.strip_suffix(')')?;
    let (mut depth, mut cur, mut out) = (0i32, String::new(), Vec::new());
    for c in inner.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur.trim().to_string());
    <[String; 3]>::try_from(out).ok()
}

/// Commande qui renomme une variable du site partout (`None` : nom vide ou inchangé).
pub fn rename_command(from: &str, to: &str) -> Option<Command> {
    let to = revel_model::slugify(to);
    (!to.is_empty() && to != from).then(|| Command::RenameVariable {
        from: from.to_string(),
        to,
    })
}

#[component]
pub fn SiteSizes(s: DocSession) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let list = Memo::new(move |_| s.doc.with(|d| d.as_ref().map(sizes).unwrap_or_default()));
    let set = move |name: String, value: Option<String>| {
        let cmd = s.doc.with_untracked(|d| {
            let d = d.as_ref()?;
            style_command(
                find_rule(d, ":root", &None),
                ":root".into(),
                None,
                vec![(format!("--{name}"), value)],
            )
        });
        if let Some(c) = cmd {
            s.apply.run(c);
        }
    };
    let rename = move |from: String, to: String| {
        if let Some(c) = rename_command(&from, &to) {
            s.apply.run(c);
        }
    };
    let add = move |_| {
        ui.ask.set(Some(crate::dialog::Ask::input(
            ui.now("sizes-add", &[]),
            ui.now("sizes-name", &[]),
            ui.now("sizes-default", &[]),
            ui.now("color-site-create", &[]),
            move |name: String| {
                let base = revel_model::slugify(&name);
                let base = if base.is_empty() {
                    "taille".to_string()
                } else {
                    base
                };
                let taken = s.doc.with_untracked(|d| {
                    d.as_ref()
                        .map(revel_edit::variables::declared)
                        .unwrap_or_default()
                });
                let name = (1..)
                    .map(|i| {
                        if i == 1 {
                            base.clone()
                        } else {
                            format!("{base}-{i}")
                        }
                    })
                    .find(|v| !taken.contains(v))
                    .unwrap_or(base);
                set(name, Some("clamp(1rem, 0.8rem + 1vw, 1.5rem)".into()));
            },
        )));
    };
    let remove = move |name: String| {
        let n = s
            .doc
            .with_untracked(|d| d.as_ref().map(|d| uses(d, &name).0).unwrap_or(0));
        let n2 = name.clone();
        ui.ask.set(Some(
            crate::dialog::Ask::confirm(
                ui.now("colors-remove-title", &[("name", name.clone())]),
                (n > 0).then(|| ui.now("colors-remove-used", &[("count", n.to_string())])),
                ui.now("action-remove", &[]),
                move || set(n2.clone(), None),
            )
            .danger(),
        ));
    };
    view! {
        <section class="section settings-card site-sizes" data-section="sizes">
            <div class="sizes-head">
                <span class="label">{ui.t("colors-name")}</span>
                <span class="label" title=ui.t("sizes-fluid-help")>{ui.t("sizes-value")}</span>
                <span></span>
            </div>
            {move || list.get().into_iter().map(|(name, value)| {
                let (n, text) = s.doc.with_untracked(|d| d.as_ref().map(|d| uses(d, &name)).unwrap_or((0, false)));
                let (n1, n2, n3, n4, n5) = (name.clone(), name.clone(), name.clone(), name.clone(), name.clone());
                let fluid = clamp_parts(&value);
                let sample = text.then(|| view! {
                    <span class="sizes-sample" style=format!("font-size:min({value}, 26px)") aria-hidden="true">"Aa"</span>
                });
                view! {
                    <div class="sizes-row" data-var=name.clone()>
                        <span class="sizes-name">
                            {sample}
                            <input class="input input--mono sizes-name-input" prop:value=name.clone() spellcheck="false"
                                aria-label=ui.t("sizes-name") title=ui.t("sizes-rename")
                                on:change=move |e| rename(n1.clone(), event_target_value(&e))/>
                        </span>
                        {match fluid {
                            Some([lo, mid, hi]) => {
                                let (mid2, hi2, lo2) = (mid.clone(), hi.clone(), lo.clone());
                                view! {
                                    <span class="sizes-fluid" title=ui.t("sizes-fluid-help")>
                                        <input class="input input--mono" prop:value=lo aria-label=ui.t("sizes-min")
                                            placeholder=ui.t("sizes-min")
                                            on:change=move |e| set(n2.clone(), Some(format!("clamp({}, {mid}, {hi2})", event_target_value(&e).trim())))/>
                                        <span class="sizes-arrow">{icons::arrow_right()}</span>
                                        <input class="input input--mono" prop:value=hi aria-label=ui.t("sizes-max")
                                            placeholder=ui.t("sizes-max")
                                            on:change=move |e| set(n3.clone(), Some(format!("clamp({lo2}, {mid2}, {})", event_target_value(&e).trim())))/>
                                    </span>
                                }.into_any()
                            }
                            None => view! {
                                <input class="input input--mono" prop:value=value aria-label=ui.t("sizes-value")
                                    on:change=move |e| { let v = event_target_value(&e).trim().to_string(); if is_size(&v) { set(n4.clone(), Some(v)) } }/>
                            }.into_any(),
                        }}
                        <span class="sizes-uses muted" title=ui.t("sizes-uses-help")>{n}</span>
                        <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                            on:click=move |_| remove(n5.clone())>{icons::trash()}</button>
                    </div>
                }
            }).collect_view()}
            {move || list.with(Vec::is_empty).then(|| view! { <p class="muted small">{ui.t("sizes-none")}</p> })}
            <div class="colors-foot">
                <button class="btn btn--small" data-action="add-size" on:click=add>{icons::plus()}<span>{ui.t("sizes-add")}</span></button>
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailles_reconnues() {
        assert!(
            is_size("1.25rem")
                && is_size("clamp(1.9rem, 1.35rem + 2.2vw, 3rem)")
                && is_size("42rem")
        );
        assert!(
            !is_size("#f4684d")
                && !is_size("linear-gradient(red, blue)")
                && !is_size("\"Montserrat\", sans-serif")
        );
        assert!(!is_size("cubic-bezier(0.2, 0.7, 0.1, 1)") && !is_size("var(--nuit)"));
        assert_eq!(
            clamp_parts("clamp(1.9rem, 1.35rem + 2.2vw, 3rem)").unwrap(),
            [
                "1.9rem".to_string(),
                "1.35rem + 2.2vw".into(),
                "3rem".into()
            ]
        );
    }
}
