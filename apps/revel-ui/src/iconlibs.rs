//! Bibliothèques d'icônes en ligne (D-205) : réglage « Icon libraries » des réglages de Revel
//! (modèle des catalogues de templates) et données partagées avec le choix d'une icône du
//! panneau Styles (menu « Library »). La liste des noms d'une bibliothèque est lue une fois ;
//! chaque dessin est lu quand il s'affiche ; les deux restent en mémoire pendant la session.

use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_protocol::{Failure, IconDrawing, IconIndex, IconLibrary};
use std::cell::RefCell;
use std::collections::HashMap;

/// Clé de la bibliothèque des icônes de Revel dans le menu « Library ».
pub(crate) const REVEL: &str = "revel";
/// Toutes les bibliothèques.
pub(crate) const ALL: &str = "all";

thread_local! {
    static INDEXES: RefCell<HashMap<String, IconIndex>> = RefCell::new(HashMap::new());
    static DRAWINGS: RefCell<HashMap<String, IconDrawing>> = RefCell::new(HashMap::new());
    /// Dernière bibliothèque choisie dans le panneau Styles.
    static LAST: RefCell<String> = RefCell::new(ALL.to_string());
    /// Recherche et nombre d'icônes montrées : gardés quand le panneau est redessiné (icône posée).
    static SEARCH: RefCell<(String, usize)> = const { RefCell::new((String::new(), 0)) };
}

pub(crate) fn search() -> (String, usize) {
    SEARCH.with(|q| q.borrow().clone())
}

pub(crate) fn set_search(q: &str, shown: usize) {
    SEARCH.with(|x| *x.borrow_mut() = (q.to_string(), shown));
}

pub(crate) fn last() -> String {
    LAST.with(|l| l.borrow().clone())
}

pub(crate) fn set_last(k: &str) {
    LAST.with(|l| *l.borrow_mut() = k.to_string());
}

/// Bibliothèques actives (réglages de Revel).
pub(crate) fn libraries(ui: Ui) -> Vec<IconLibrary> {
    ui.settings.with(|s| s.icon_libraries.clone())
}

/// Noms d'une bibliothèque déjà lus pendant la session.
pub(crate) fn cached_index(lib: &IconLibrary) -> Option<IconIndex> {
    INDEXES.with(|m| m.borrow().get(&lib.key()).cloned())
}

/// Noms d'une bibliothèque (lus une fois par session ; gardés sur l'ordinateur par Revel).
pub(crate) async fn index(lib: &IconLibrary, refresh: bool) -> Result<IconIndex, Failure> {
    if !refresh && let Some(i) = cached_index(lib) {
        return Ok(i);
    }
    let i = api::icon_library(lib, refresh).await?;
    INDEXES.with(|m| m.borrow_mut().insert(lib.key(), i.clone()));
    Ok(i)
}

/// Dessin déjà lu.
pub(crate) fn cached_drawing(lib: &IconLibrary, name: &str) -> Option<IconDrawing> {
    DRAWINGS.with(|m| m.borrow().get(&format!("{}:{name}", lib.key())).cloned())
}

/// Lit les dessins manquants de `names` (une seule demande pour le lot).
pub(crate) async fn load_drawings(lib: &IconLibrary, names: &[String]) {
    let missing: Vec<String> = names
        .iter()
        .filter(|n| cached_drawing(lib, n).is_none())
        .cloned()
        .collect();
    if missing.is_empty() {
        return;
    }
    if let Ok(list) = api::icon_drawings(lib, &missing).await {
        DRAWINGS.with(|m| {
            let mut m = m.borrow_mut();
            for d in list {
                m.insert(format!("{}:{}", lib.key(), d.name), d);
            }
        });
    }
}

/// `<svg>` d'un dessin (contenu déjà nettoyé par Revel, peinture aux valeurs sûres).
pub(crate) fn svg_markup(d: &IconDrawing, size: u32) -> String {
    let esc = |v: &str| v.replace(['"', '<', '>', '&'], "");
    let mut paint = String::new();
    let has_fill = d.paint.iter().any(|(k, _)| k == "fill");
    for (k, v) in &d.paint {
        paint.push_str(&format!(" {k}=\"{}\"", esc(v)));
    }
    if !has_fill {
        paint.push_str(" fill=\"currentColor\"");
    }
    format!(
        "<svg viewBox=\"{}\" width=\"{size}\" height=\"{size}\" aria-hidden=\"true\"{paint}>{}</svg>",
        esc(&d.view_box),
        d.inner
    )
}

/// Nom d'une icône choisie dans une bibliothèque (`data-icon`) : `dépôt/dossier:nom`.
pub(crate) fn icon_name(lib: &IconLibrary, name: &str) -> String {
    format!("{}:{name}", lib.key())
}

/// Bibliothèque et nom d'une icône posée (`data-icon`), `None` pour une icône de Revel ou un
/// fichier importé.
pub(crate) fn parse_icon_name(v: &str) -> Option<(String, String)> {
    if v.starts_with("file:") {
        return None;
    }
    let (k, n) = v.rsplit_once(':')?;
    Some((k.to_string(), n.to_string()))
}

/// Ouvre les réglages de Revel sur « Icon libraries » (« Manage libraries… »).
pub(crate) fn open_manage(ui: Ui) {
    ui.route.set(crate::Route::Settings);
    spawn_local(async move {
        crate::sleep_ms(60).await;
        crate::site::scroll_to_part("icon-libraries");
    });
}

/// Nom proposé pour un dépôt ajouté : son nom, sans « -icons », avec une capitale.
fn name_of(repo: &str) -> String {
    let n = repo.rsplit('/').next().unwrap_or(repo);
    let n = n
        .trim_end_matches("-icons")
        .trim_start_matches("icons-")
        .replace(['-', '_'], " ");
    let mut c = n.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}

/// `propriétaire/nom` d'une saisie (adresse GitHub ou `propriétaire/nom`).
fn repo_of(text: &str) -> Option<String> {
    let t = text
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("github.com/")
        .trim_end_matches(".git")
        .trim_matches('/');
    let mut it = t.split('/');
    let (o, n) = (it.next()?, it.next()?);
    let ok = |s: &str| {
        !s.is_empty()
            && !s.starts_with('.')
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    (ok(o) && ok(n)).then(|| format!("{o}/{n}"))
}

/// Ligne d'une bibliothèque : état de la liste des noms.
#[derive(Clone, PartialEq)]
enum State {
    Loading,
    Ready(usize, bool),
    Failed(String),
}

/// Réglages de Revel › Icon libraries : bibliothèques actives (nom, dépôt, dossier, licence,
/// nombre d'icônes, retrait), bibliothèques proposées ajoutées en un clic, ajout d'un dépôt.
#[component]
pub fn IconLibrariesCard() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let libs = move || libraries(ui);
    let states = RwSignal::new(HashMap::<String, State>::new());
    let check = move |lib: IconLibrary, refresh: bool| {
        let k = lib.key();
        states.update(|m| {
            m.insert(k.clone(), State::Loading);
        });
        spawn_local(async move {
            let st = match index(&lib, refresh).await {
                Ok(i) => {
                    // Licence lue sur le dépôt quand elle n'était pas connue.
                    if lib.license.is_empty() && !i.license.is_empty() {
                        let (key, lic) = (lib.key(), i.license.clone());
                        crate::settings::update(ui, &|s| {
                            if let Some(l) = s.icon_libraries.iter_mut().find(|l| l.key() == key) {
                                l.license = lic.clone();
                            }
                        });
                    }
                    State::Ready(i.names.len(), i.offline)
                }
                Err(e) => State::Failed(ui.failure(&e)),
            };
            let _ = states.try_update(|m| {
                m.insert(k, st);
            });
        });
    };
    for l in libs() {
        check(l, false);
    }
    let suggested = move || {
        let on = libs();
        revel_protocol::suggested_icon_libraries()
            .into_iter()
            .filter(|s| !on.iter().any(|l| l.key() == s.key()))
            .collect::<Vec<_>>()
    };
    let add_lib = move |lib: IconLibrary| {
        let key = lib.key();
        let l2 = lib.clone();
        crate::settings::update(ui, &|s| {
            if !s.icon_libraries.iter().any(|l| l.key() == key) {
                s.icon_libraries.push(l2.clone());
            }
        });
        check(lib, false);
    };
    let repo = RwSignal::new(String::new());
    let dir = RwSignal::new(String::new());
    let bad = RwSignal::new(false);
    let add = move || {
        let Some(r) = repo_of(&repo.get_untracked()) else {
            bad.set(true);
            return;
        };
        let d = dir.get_untracked().trim().trim_matches('/').to_string();
        bad.set(false);
        repo.set(String::new());
        dir.set(String::new());
        add_lib(IconLibrary {
            name: name_of(&r),
            repo: r,
            dir: d,
            license: String::new(),
        });
    };
    let revel_count = revel_edit::icons::ICONS.len();
    view! {
        <section class="section settings-card" data-section="icon-libraries">
            <h2>{ui.t("icon-libraries")}</h2>
            <div class="form-grid">
                <span class="label" title=ui.t("icon-libraries-list-help")>{ui.t("icon-libraries-list")}</span>
                <div class="catalog-list iconlib-list">
                    <div class="catalog-row iconlib-row" data-lib=REVEL>
                        <span class="iconlib-logo">{icons::logo()}</span>
                        <span class="iconlib-name"><b>"Revel"</b><span class="mono muted">{ui.t("icon-library-builtin")}</span></span>
                        <span></span>
                        <span class="catalog-state">{ui.tx("icon-library-count", &[("n", fmt_count(ui, revel_count))])}</span>
                        <span></span>
                    </div>
                    <For each=libs key=IconLibrary::key let:lib>
                        {
                            let k = lib.key();
                            let (k1, k2) = (k.clone(), k.clone());
                            let l2 = lib.clone();
                            let place = if lib.dir.is_empty() { lib.repo.clone() } else { format!("{} · {}", lib.repo, lib.dir) };
                            view! {
                                <div class="catalog-row iconlib-row" data-lib=k.clone()>
                                    {icons::cloud()}
                                    <span class="iconlib-name" title=format!("github.com/{}", lib.repo)>
                                        <b>{lib.name.clone()}</b><span class="mono muted">{place}</span>
                                    </span>
                                    {(!lib.license.is_empty()).then(|| view! { <span class="chip iconlib-license" title=ui.t("icon-library-license")>{lib.license.clone()}</span> })}
                                    <span class="catalog-state" role="status">
                                        {move || match states.with(|m| m.get(&k1).cloned()) {
                                            None | Some(State::Loading) => view! { <span class="muted">"…"</span> }.into_any(),
                                            Some(State::Ready(n, offline)) => view! {
                                                <span class="catalog-ok" class:is-offline=offline
                                                    title=move || if offline { ui.t("icon-library-offline") } else { ui.t("icon-library-ready") }>
                                                    {icons::cloud()}{ui.tx("icon-library-count", &[("n", fmt_count(ui, n))])}
                                                </span>
                                            }.into_any(),
                                            Some(State::Failed(why)) => {
                                                let l3 = l2.clone();
                                                view! {
                                                    <button type="button" class="catalog-missing iconlib-retry" title=why
                                                        on:click=move |_| check(l3.clone(), true)>
                                                        {icons::warning()}{ui.t("icon-library-failed")}
                                                    </button>
                                                }.into_any()
                                            }
                                        }}
                                    </span>
                                    <button type="button" class="icon-btn icon-btn--small catalog-remove" data-action="remove-library"
                                        title=ui.t("icon-library-remove") aria-label=ui.t("icon-library-remove")
                                        on:click=move |_| {
                                            let k = k2.clone();
                                            crate::settings::update(ui, &|s| s.icon_libraries.retain(|l| l.key() != k));
                                        }>{icons::trash()}</button>
                                </div>
                            }
                        }
                    </For>
                    <For each=suggested key=IconLibrary::key let:lib>
                        {
                            let l2 = lib.clone();
                            let place = format!("{} · {}", lib.repo, lib.dir);
                            view! {
                                <div class="catalog-row iconlib-row is-suggested" data-lib=lib.key()>
                                    {icons::cloud()}
                                    <span class="iconlib-name" title=format!("github.com/{}", lib.repo)>
                                        <b>{lib.name.clone()}</b><span class="mono muted">{place}</span>
                                    </span>
                                    <span class="chip iconlib-license" title=ui.t("icon-library-license")>{lib.license.clone()}</span>
                                    <span></span>
                                    <button type="button" class="btn btn--small iconlib-add" data-action="add-library"
                                        title=ui.t("icon-library-add-help")
                                        on:click=move |_| add_lib(l2.clone())>{icons::plus()}{ui.t("icon-library-add")}</button>
                                </div>
                            }
                        }
                    </For>
                    <div class="catalog-add iconlib-new">
                        <input class="input mono" type="text" spellcheck="false" autocomplete="off"
                            placeholder=ui.t("template-catalog-add-placeholder") aria-label=ui.t("icon-library-repo")
                            aria-invalid=move || bad.get().to_string()
                            prop:value=move || repo.get() on:input=move |e| { repo.set(event_target_value(&e)); bad.set(false) }
                            on:keydown=move |e| if e.key() == "Enter" { add() }/>
                        <input class="input mono" type="text" spellcheck="false" autocomplete="off"
                            placeholder=ui.t("icon-library-dir-placeholder") aria-label=ui.t("icon-library-dir")
                            prop:value=move || dir.get() on:input=move |e| dir.set(event_target_value(&e))
                            on:keydown=move |e| if e.key() == "Enter" { add() }/>
                        <button type="button" class="btn btn--small" data-action="add-repo" on:click=move |_| add()
                            disabled=move || repo.with(|d| d.trim().is_empty())>{icons::plus()}{ui.t("icon-library-add")}</button>
                    </div>
                    {move || bad.get().then(|| view! { <span class="field-error" role="alert">{ui.t("template-catalog-invalid")}</span> })}
                </div>
            </div>
        </section>
    }
}

/// Nombre mis en forme par la langue de l'interface (« 1,900 », « 1 900 »).
pub(crate) fn fmt_count(ui: Ui, n: usize) -> String {
    let lang = ui.lang();
    let locales = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(&lang));
    let f = js_sys::Intl::NumberFormat::new(&locales, &js_sys::Object::new());
    f.format()
        .call1(
            &wasm_bindgen::JsValue::NULL,
            &wasm_bindgen::JsValue::from_f64(n as f64),
        )
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depot_et_nom() {
        assert_eq!(
            repo_of("https://github.com/tabler/tabler-icons"),
            Some("tabler/tabler-icons".into())
        );
        assert_eq!(
            repo_of("lucide-icons/lucide"),
            Some("lucide-icons/lucide".into())
        );
        assert_eq!(repo_of("pas un dépôt"), None);
        assert_eq!(name_of("tabler/tabler-icons"), "Tabler");
        assert_eq!(name_of("someone/my_icons"), "My icons");
        assert_eq!(
            parse_icon_name("lucide-icons/lucide/icons:arrow-right"),
            Some(("lucide-icons/lucide/icons".into(), "arrow-right".into()))
        );
        assert_eq!(parse_icon_name("file:/media/a.svg"), None);
        assert_eq!(parse_icon_name("etoile"), None);
    }
}
