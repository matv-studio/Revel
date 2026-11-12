//! Flux reliés aux collections (D-167) : bouton « Flux » de l'en-tête d'une collection, menu
//! flottant (adresse du flux, Reprendre, Délier, date de la dernière reprise, bilan).

use crate::{Ui, api, icons};
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::Command;
use revel_model::FeedSource;

/// État du menu des flux, gardé par la vue CMS : l'en-tête de la collection est redessiné à
/// chaque relecture des contenus, le menu ouvert et son bilan restent.
#[derive(Clone, Copy)]
pub struct FeedState {
    open: RwSignal<bool>,
    sources: RwSignal<Vec<FeedSource>>,
    /// Flux de la collection en cours de réglage : (origine, adresse).
    rows: RwSignal<Vec<(String, String)>>,
    busy: RwSignal<bool>,
    report: RwSignal<Option<revel_protocol::FeedSyncReport>>,
    error: RwSignal<Option<String>>,
    /// Collection dont les lignes sont chargées.
    loaded: RwSignal<Option<String>>,
    /// Dernière révision du document vue.
    seen: RwSignal<Option<u64>>,
}

impl Default for FeedState {
    fn default() -> Self {
        Self::new()
    }
}

impl FeedState {
    pub fn new() -> Self {
        Self {
            open: RwSignal::new(false),
            sources: RwSignal::new(Vec::new()),
            rows: RwSignal::new(Vec::new()),
            busy: RwSignal::new(false),
            report: RwSignal::new(None),
            error: RwSignal::new(None),
            loaded: RwSignal::new(None),
            seen: RwSignal::new(None),
        }
    }
}

#[component]
pub fn FeedButton(
    state: FeedState,
    /// Dossier du site.
    path: StoredValue<String>,
    /// Collection affichée (identifiant).
    collection: Signal<Option<String>>,
    /// Compteur augmenté après une reprise ou un réglage : la vue relit contenus et historique.
    done: RwSignal<u32>,
    /// Désactivé (langue autre que la principale).
    disabled: Signal<bool>,
    /// Révision du document : flux relus quand elle change (annuler, rétablir).
    revision: Signal<Option<u64>>,
) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let FeedState {
        open,
        sources,
        rows,
        busy,
        report,
        error,
        loaded,
        seen,
    } = state;
    let mine = move || -> Vec<FeedSource> {
        let Some(id) = collection.get() else {
            return Vec::new();
        };
        sources.with(|s| s.iter().filter(|f| f.collection == id).cloned().collect())
    };
    // `id` : collection, lue par l'appelant avant toute attente (son signal disparaît quand
    // l'en-tête est redessiné).
    let load = move |id: Option<String>| {
        let Some(p) = path.try_get_value() else {
            return;
        };
        spawn_local(async move {
            if let Ok(o) = api::edit_open(&p).await
                && !sources.is_disposed()
            {
                let list = o.site.settings.feed_sources;
                let mut r: Vec<(String, String)> = list
                    .iter()
                    .filter(|f| Some(&f.collection) == id.as_ref())
                    .map(|f| (f.source.clone(), f.url.clone()))
                    .collect();
                if r.is_empty() {
                    r.push((String::new(), String::new()));
                }
                rows.set(r);
                sources.set(list);
            }
        });
    };
    // Document modifié ailleurs (annuler, rétablir) pendant que le menu est fermé : flux relus.
    // Dernière révision vue gardée dans l'état (le bouton est recréé à chaque relecture).
    Effect::new(move |_| {
        let rev = revision.get();
        if seen.get_untracked() != rev {
            let first = seen.get_untracked().is_none();
            seen.set(rev);
            if !first && !open.get_untracked() && !busy.get_untracked() {
                load(collection.get_untracked());
            }
        }
    });
    Effect::new(move |_| {
        let id = collection.get();
        if loaded.get_untracked() != id {
            loaded.set(id.clone());
            open.set(false);
            report.set(None);
            error.set(None);
            load(id);
        }
    });
    // Flux de la collection remplacés par les lignes (lignes sans adresse ignorées).
    let with_rows = move |id: &str, r: &[(String, String)]| -> Vec<FeedSource> {
        let old = sources.get_untracked();
        let mut all: Vec<FeedSource> = old.iter().filter(|f| f.collection != id).cloned().collect();
        for (src, u) in r {
            let u = u.trim().to_string();
            if u.is_empty() {
                continue;
            }
            let synced = old
                .iter()
                .find(|f| f.collection == id && f.url == u)
                .and_then(|f| f.synced.clone());
            all.push(FeedSource {
                collection: id.to_string(),
                url: u,
                source: src.trim().to_string(),
                synced,
            });
        }
        all
    };
    let save = move |then_sync: bool| {
        let (Some(p), Some(id)) = (path.try_get_value(), collection.get_untracked()) else {
            return;
        };
        let r = rows.get_untracked();
        if r.iter().any(|(_, u)| {
            let u = u.trim();
            !u.is_empty() && !(u.starts_with("https://") || u.starts_with("http://"))
        }) {
            error.set(Some(ui.now("cms-feed-url-bad", &[])));
            return;
        }
        let list = with_rows(&id, &r);
        let changed = list != sources.get_untracked();
        let any = list.iter().any(|f| f.collection == id);
        busy.set(then_sync);
        error.set(None);
        report.set(None);
        let id2 = id.clone();
        spawn_local(async move {
            let res = async {
                if changed {
                    api::edit_apply(&p, &Command::SetFeedSources { sources: list }, false).await?;
                }
                if then_sync && any {
                    return api::feed_sync(&p, &id).await.map(Some);
                }
                Ok(None)
            }
            .await;
            if busy.is_disposed() {
                return;
            }
            busy.set(false);
            match res {
                Ok(rep) => report.set(rep),
                Err(e) => error.set(Some(ui.failure(&e))),
            }
            load(Some(id2));
            done.update(|n| *n += 1);
        });
    };
    // Fermé par un clic ailleurs ou Échap (lignes enregistrées en fermant).
    let root = NodeRef::<leptos::html::Div>::new();
    let close = move || {
        if open.get_untracked() {
            open.set(false);
            save(false);
        }
    };
    let on_down = window_event_listener(ev::pointerdown, move |e: web_sys::PointerEvent| {
        if !open.get_untracked() {
            return;
        }
        let inside = root.get_untracked().zip(e.target()).is_some_and(|(r, t)| {
            use wasm_bindgen::JsCast;
            t.dyn_ref::<web_sys::Node>()
                .is_some_and(|n| r.contains(Some(n)))
        });
        if !inside {
            close();
        }
    });
    let on_key = window_event_listener(ev::keydown, move |e: web_sys::KeyboardEvent| {
        if open.get_untracked() && e.key() == "Escape" {
            e.stop_propagation();
            close();
        }
    });
    on_cleanup(move || {
        on_down.remove();
        on_key.remove();
    });
    let row_view = move |i: usize| {
        let get = move |f: fn(&(String, String)) -> String| {
            move || rows.with(|r| r.get(i).map(f).unwrap_or_default())
        };
        view! {
            <div class="cms-feed-row" data-role="feed-row">
                <input class="input cms-feed-source" type="text" data-role="feed-source" spellcheck="false"
                    placeholder=ui.t("cms-feed-source") aria-label=ui.t("cms-feed-source") title=ui.t("cms-feed-source-help")
                    prop:value=get(|r| r.0.clone())
                    on:input=move |e| { let v = event_target_value(&e); rows.update(|r| if let Some(x) = r.get_mut(i) { x.0 = v }) }/>
                <input class="input cms-feed-url" type="url" data-role="feed-url" spellcheck="false"
                    placeholder="https://exemple.fr/feed.xml" aria-label=ui.t("cms-feed-url")
                    prop:value=get(|r| r.1.clone())
                    on:input=move |e| { let v = event_target_value(&e); error.set(None); rows.update(|r| if let Some(x) = r.get_mut(i) { x.1 = v }) }
                    on:keydown=move |e: ev::KeyboardEvent| if e.key() == "Enter" { e.prevent_default(); save(true) }/>
                <button type="button" class="icon-btn icon-btn--small" data-action="feed-remove"
                    title=ui.t("cms-feed-unlink") aria-label=ui.t("cms-feed-unlink")
                    on:click=move |_| rows.update(|r| {
                        if i < r.len() { r.remove(i); }
                        if r.is_empty() { r.push((String::new(), String::new())) }
                    })>
                    {icons::close()}
                </button>
            </div>
        }
    };
    view! {
        <div class="cms-feed" node_ref=root>
            <button type="button" class="btn cms-feed-btn" data-action="cms-feed"
                class:is-on=move || !mine().is_empty()
                aria-expanded=move || open.get().to_string()
                title=move || ui.tx(if disabled.get() { "cms-locale-shared" } else { "cms-feed-help" }, &[])
                prop:disabled=move || disabled.get()
                on:click=move |_| if open.get_untracked() { close() } else { open.set(true) }>
                {icons::rss()}<span class="btn-label">{ui.t("cms-feed")}</span>
                {move || { let n = mine().len(); (n > 1).then(|| view! { <span class="cms-feed-count">{n}</span> }) }}
            </button>
            <Show when=move || open.get()>
                <div class="pop cms-feed-pop" role="dialog" aria-label=ui.t("cms-feed")>
                    <div class="cms-feed-head">
                        <span class="label" title=ui.t("cms-feed-source-help")>{ui.t("cms-feed-source")}</span>
                        <span class="label">{ui.t("cms-feed-url")}</span>
                    </div>
                    {move || (0..rows.with(Vec::len)).map(row_view).collect_view()}
                    <button type="button" class="btn btn--small cms-feed-add" data-action="feed-add"
                        on:click=move |_| rows.update(|r| r.push((String::new(), String::new())))>
                        {icons::plus()}<span>{ui.t("cms-feed-add")}</span>
                    </button>
                    {move || error.get().map(|m| view! { <p class="cms-feed-error" role="alert">{m}</p> })}
                    {move || report.get().map(|r| view! {
                        <div class="cms-feed-report" data-role="feed-report">
                            <p class="status--ok">
                                {icons::check()}
                                <span>{ui.tx("cms-feed-report", &[("created", r.created.to_string()), ("updated", r.updated.to_string())])}</span>
                                {(r.duplicates > 0).then(|| view! { <span class="muted">{ui.tx("cms-feed-duplicates", &[("count", r.duplicates.to_string())])}</span> })}
                            </p>
                            {r.errors.into_iter().map(|(u, d)| { let tip = d.clone(); view! {
                                <p class="cms-feed-error" title=tip>{ui.tx("cms-feed-failed", &[("url", u), ("detail", d)])}</p>
                            }}).collect_view()}
                        </div>
                    })}
                    <div class="cms-feed-actions">
                        {move || mine().iter().filter_map(|f| f.synced.clone()).max().map(|d| view! {
                            <span class="cms-feed-date muted" title=ui.tx("cms-feed-synced", &[])>{icons::sync()}<span>{d}</span></span>
                        })}
                        <span class="cms-feed-gap"></span>
                        <button type="button" class="btn btn--primary btn--small" data-action="feed-sync"
                            prop:disabled=move || busy.get() || rows.with(|r| r.iter().all(|(_, u)| u.trim().is_empty()))
                            on:click=move |_| save(true)>
                            {move || if busy.get() { ui.tx("cms-feed-syncing", &[]) } else { ui.tx("cms-feed-sync", &[]) }}
                        </button>
                    </div>
                </div>
            </Show>
        </div>
    }
}
