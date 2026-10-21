//! Réglages lisibles (3.12, D-28) : calculs des gabarits présentés comme réglages simples —
//! titre de l'onglet, image de partage par défaut et adresse officielle (Paramètres ›
//! Référencement), menus du site (Paramètres › Navigation), contenus mis en avant (paramètres
//! de la page). La reconnaissance est faite par `revel_edit::readable`, les valeurs des données
//! sont lues par le cœur ; chaque modification est une commande annulable.

use crate::inspector::Field;
use crate::site::DocSession;
use crate::{Ui, api, icons};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::Command;
use revel_edit::canvas::{Binding, Seg, Source};
use revel_edit::readable::{self as rd, Link, TitleFormat, TitleSetting};
use revel_model::Site;
use revel_protocol::{PickEntry, ReadableData};
use serde_json::Value;

/// Séparateurs proposés pour le titre de l'onglet.
const SEPS: &[&str] = &[" | ", " — ", " · ", " - ", " : "];

/// Commande qui écrit une valeur de données (`None` : clé retirée).
pub fn data_command(path: &[String], value: Option<Value>) -> Command {
    Command::SetDataValue {
        binding: Binding {
            source: Source::Data,
            path: path.iter().map(|k| Seg::Key(k.clone())).collect(),
        },
        value,
    }
}

/// Données lues par le cœur pour des chemins et des collections, relues quand le document change.
fn load(
    ui: Ui,
    path: String,
    doc: Signal<Option<Site>>,
    query: impl Fn(&Site) -> (Vec<Vec<String>>, Vec<String>) + Send + Sync + 'static,
) -> RwSignal<Option<(Vec<Vec<String>>, ReadableData)>> {
    let out = RwSignal::new(None);
    let path = StoredValue::new(path);
    Effect::new(move |_| {
        let Some((paths, cols)) = doc.with(|d| d.as_ref().map(&query)) else {
            return;
        };
        let p = path.get_value();
        spawn_local(async move {
            match api::readable_data(&p, &paths, &cols).await {
                Ok(d) => {
                    let _ = out.try_set(Some((paths, d)));
                }
                Err(e) => ui.fail(e),
            }
        });
    });
    out
}

/// Texte d'une valeur de données.
fn text(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or_default().to_string()
}

/// Paramètres › Référencement : titre de l'onglet, image de partage par défaut, adresse
/// officielle (seulement ce que les gabarits du site permettent de régler).
#[component]
pub fn SeoReadable(s: DocSession, path: String) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let title = Memo::new(move |_| s.doc.with(|d| d.as_ref().and_then(rd::title)));
    let share = Memo::new(move |_| s.doc.with(|d| d.as_ref().and_then(rd::share_image)));
    let canon = Memo::new(move |_| s.doc.with(|d| d.as_ref().and_then(rd::canonical_base)));
    let data = load(ui, path, s.doc.into(), |d| {
        let mut v = Vec::new();
        v.extend(rd::title(d).map(|t| t.site_path));
        v.extend(rd::share_image(d).map(|u| u.path));
        v.extend(rd::canonical_base(d).map(|u| u.path));
        (v, Vec::new())
    });
    let value_of = move |p: &[String]| {
        data.with(|d| {
            d.as_ref().and_then(|(paths, rd)| {
                let i = paths.iter().position(|x| x == p)?;
                rd.values.get(i).cloned().flatten()
            })
        })
    };
    let set_title = move |t: TitleSetting, f: TitleFormat| {
        if let Some(cmd) = s
            .doc
            .with_untracked(|d| d.as_ref().and_then(|d| t.command(d, &f)))
        {
            s.apply.run(cmd);
        }
    };
    // Exemple : une page du site (son nom), sinon un titre générique.
    let sample_page = move || {
        s.doc
            .with(|d| {
                d.as_ref()?
                    .pages
                    .iter()
                    .find(|p| {
                        p.path != "/"
                            && p.path.ends_with('/')
                            && !p.path.contains(['{', '.'])
                            && !p.draft
                    })
                    .map(|p| {
                        p.name.clone().unwrap_or_else(|| {
                            let last = p
                                .path
                                .trim_matches('/')
                                .rsplit('/')
                                .next()
                                .unwrap_or_default();
                            let mut c = last.replace('-', " ").chars().collect::<Vec<_>>();
                            if let Some(f) = c.first_mut() {
                                *f = f.to_ascii_uppercase();
                            }
                            c.into_iter().collect()
                        })
                    })
            })
            .unwrap_or_else(|| ui.now("seo-title-sample", &[]))
    };
    move || {
        let (t, sh, ca) = (title.get(), share.get(), canon.get());
        (t.is_some() || sh.is_some() || ca.is_some()).then(|| {
            view! {
                <div class="section readable">
                    <h2 title=ui.t("seo-readable-help")>{ui.t("seo-readable")}</h2>
                    {t.map(|t| {
                        let f = t.format.clone();
                        let site_name = text(value_of(&t.site_path).as_ref());
                        let (t1, t2) = (t.clone(), t.clone());
                        let (f1, f2) = (f.clone(), f.clone());
                        let mode = if f.title_only { "only" } else if f.site_first { "site" } else { "title" };
                        let sample = TitleSetting::sample(&f, &sample_page(), &site_name);
                        view! {
                            <div class="field readable-title">
                                <span class="label" title=ui.t("seo-tab-title-help")>{ui.t("seo-tab-title")}</span>
                                <div class="readable-row">
                                    <div class="segmented" role="radiogroup" aria-label=ui.t("seo-tab-title")>
                                        {[("title", "seo-title-first"), ("site", "seo-site-first"), ("only", "seo-title-only")].into_iter().map(|(m, key)| {
                                            let (t, f) = (t1.clone(), f1.clone());
                                            view! {
                                                <button role="radio" class:is-on=mode == m aria-checked=(mode == m).to_string() data-mode=m
                                                    on:click=move |_| set_title(t.clone(), TitleFormat {
                                                        title_only: m == "only",
                                                        site_first: m == "site",
                                                        sep: f.sep.clone(),
                                                    })>{ui.t(key)}</button>
                                            }
                                        }).collect_view()}
                                    </div>
                                    <div class="select readable-sep" class:is-disabled=f.title_only title=ui.t("seo-title-sep")>
                                        <select class="input input--mono" aria-label=ui.t("seo-title-sep") disabled=f.title_only
                                            on:change=move |e| {
                                                let sep = event_target_value(&e);
                                                set_title(t2.clone(), TitleFormat { sep, ..f2.clone() })
                                            }>
                                            {SEPS.iter().chain((!SEPS.contains(&f.sep.as_str())).then_some(&f.sep.as_str())).map(|sp| {
                                                let sp = sp.to_string();
                                                view! { <option value=sp.clone() selected=sp == f.sep>{format!("« {sp} »")}</option> }
                                            }).collect_view()}
                                        </select>
                                    </div>
                                </div>
                                <span class="readable-sample" title=ui.t("seo-title-sample-help")>{sample}</span>
                            </div>
                        }
                    })}
                    {sh.map(|u| {
                        let p1 = u.path.clone();
                        let p2 = u.path.clone();
                        let cur = Signal::derive({ let p = u.path.clone(); move || text(value_of(&p).as_ref()) });
                        view! {
                            <Field label=ui.t("seo-share-default") value=cur placeholder="/media/partage.jpg"
                                on_commit=move |v: String| s.apply.run(data_command(&p1, (!v.trim().is_empty()).then(|| Value::String(v.trim().to_string()))))>
                                <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-image") aria-label=ui.t("picker-choose-image")
                                    on:click=move |_| {
                                        let p = p2.clone();
                                        let shelf = s.doc.with_untracked(|d| d.as_ref().map(|d| d.library.media.clone()));
                                        crate::library::pick_image(ui, shelf, move |url| s.apply.run(data_command(&p, Some(Value::String(url)))))
                                    }>{icons::image()}</button>
                            </Field>
                        }
                    })}
                    {ca.map(|u| {
                        let p = u.path.clone();
                        let cur = Signal::derive({ let p = u.path.clone(); move || text(value_of(&p).as_ref()) });
                        view! {
                            <div title=ui.t("seo-canonical-base-help")>
                                <Field label=ui.t("seo-canonical-base") value=cur mono=true placeholder="https://exemple.com"
                                    on_commit=move |v: String| {
                                        let v = v.trim().trim_end_matches('/').to_string();
                                        if !(v.starts_with("https://") || v.starts_with("http://")) {
                                            ui.fail(revel_protocol::Failure::new("seo-canonical-invalid"));
                                            return;
                                        }
                                        s.apply.run(data_command(&p, Some(Value::String(v))))
                                    }/>
                            </div>
                        }
                    })}
                </div>
            }
        })
    }
}

/// Paramètres › Navigation : listes de liens des gabarits (menu du site), modifiables comme des
/// listes : texte, adresse (pages du site proposées), ordre, ajout, retrait.
#[component]
pub fn Menus(s: DocSession) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let lists = Memo::new(move |_| {
        s.doc
            .with(|d| d.as_ref().map(rd::link_lists).unwrap_or_default())
    });
    let pages = Memo::new(move |_| {
        s.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    d.pages
                        .iter()
                        .filter(|p| !p.path.contains('{') && !p.draft)
                        .map(|p| p.path.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let owner_name = move |o: &revel_edit::Owner| {
        s.doc.with_untracked(|d| match o {
            revel_edit::Owner::Component(id) => d
                .as_ref()
                .and_then(|d| d.component(id).map(crate::names::component_name))
                .unwrap_or_else(|| id.clone()),
            revel_edit::Owner::Layout(id) | revel_edit::Owner::Page(id) => id.clone(),
        })
    };
    view! {
        // Aucune liste de liens dans les gabarits : rien (partie retirée des paramètres).
        {move || (!lists.with(Vec::is_empty)).then(|| view! {
        <section class="page settings-page">
            <header class="page-head"><h1 title=ui.t("menus-help")>{ui.t("settings-menus")}</h1></header>
            {move || {
                let ls = lists.get();
                ls.into_iter().enumerate().map(|(n, l)| {
                    let list = StoredValue::new(l.clone());
                    let write = move |links: Vec<Link>| s.apply.run(list.with_value(|l| l.command(&links)));
                    let links = l.links.clone();
                    let count = links.len();
                    let dl = format!("menu-pages-{n}");
                    view! {
                        <div class="section navlist" data-navlist=l.name.clone()>
                            <h2 class="navlist-head">
                                <span>{owner_name(&l.owner)}</span>
                                <code class="code-tag">{l.name.clone()}</code>
                            </h2>
                            <datalist id=dl.clone()>
                                {move || pages.get().into_iter().map(|p| view! { <option value=p></option> }).collect_view()}
                            </datalist>
                            <ul class="navlist-links">
                                {links.into_iter().enumerate().map(|(i, link)| {
                                    let edit = move |f: &dyn Fn(&mut Link)| {
                                        let mut all = list.with_value(|l| l.links.clone());
                                        if let Some(x) = all.get_mut(i) {
                                            f(x);
                                        }
                                        write(all);
                                    };
                                    let shift = move |d: isize| {
                                        let mut all = list.with_value(|l| l.links.clone());
                                        let j = i as isize + d;
                                        if j >= 0 && (j as usize) < all.len() {
                                            all.swap(i, j as usize);
                                            write(all);
                                        }
                                    };
                                    view! {
                                        <li class="navlist-link">
                                            <input class="input navlist-label" aria-label=ui.t("menus-label") placeholder=ui.t("menus-label")
                                                prop:value=link.label.clone()
                                                on:change=move |e| { let v = event_target_value(&e); edit(&|x| x.label = v.trim().to_string()) }/>
                                            <input class="input input--mono navlist-url" aria-label=ui.t("menus-url") placeholder="/page/" list=dl.clone()
                                                prop:value=link.url.clone()
                                                on:change=move |e| { let v = event_target_value(&e); edit(&|x| x.url = v.trim().to_string()) }/>
                                            <button class="icon-btn icon-btn--small" title=ui.t("layers-up") aria-label=ui.t("layers-up")
                                                disabled=i == 0 on:click=move |_| shift(-1)>{icons::arrow_up()}</button>
                                            <button class="icon-btn icon-btn--small" title=ui.t("layers-down") aria-label=ui.t("layers-down")
                                                disabled=i + 1 == count on:click=move |_| shift(1)>{icons::arrow_down()}</button>
                                            <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                                                on:click=move |_| {
                                                    let mut all = list.with_value(|l| l.links.clone());
                                                    all.remove(i);
                                                    write(all);
                                                }>{icons::trash()}</button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                            <button class="btn btn--small navlist-add" on:click=move |_| {
                                let mut all = list.with_value(|l| l.links.clone());
                                all.push(Link { label: ui.now("menus-new", &[]), url: "/".into() });
                                write(all);
                            }>{icons::plus()}{ui.t("menus-add")}</button>
                        </div>
                    }
                }).collect_view()
            }}
        </section>
        })}
    }
}

/// Paramètres de la page : listes de contenus mis en avant lues par la page (identifiants dans
/// les données), choisies parmi les contenus des collections, dans l'ordre.
#[component]
pub fn PagePicks(page: String) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<crate::editor::Ed>();
    let page = StoredValue::new(page);
    let picks = Memo::new(move |_| {
        ed.doc.with(|d| {
            d.as_ref()
                .map(|d| {
                    rd::picks(d)
                        .into_iter()
                        .filter(|p| {
                            page.with_value(|id| p.owner == revel_edit::Owner::Page(id.clone()))
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let doc: Signal<Option<Site>> = ed.doc.into();
    let data = load(ui, ed.path.get_value(), doc, move |d| {
        let ps: Vec<_> = rd::picks(d)
            .into_iter()
            .filter(|p| page.with_value(|id| p.owner == revel_edit::Owner::Page(id.clone())))
            .collect();
        let mut cols: Vec<String> = Vec::new();
        for p in &ps {
            for c in &p.collections {
                if !cols.contains(c) {
                    cols.push(c.clone());
                }
            }
        }
        (ps.into_iter().map(|p| p.path).collect(), cols)
    });
    let title_of = |entries: &[PickEntry], slug: &str| {
        entries
            .iter()
            .find(|e| e.slug == slug)
            .map(|e| (e.title.clone(), e.collection.clone()))
    };
    move || {
        let Some((paths, d)) = data.get() else {
            return ().into_any();
        };
        let shown: Vec<_> = picks
            .get()
            .into_iter()
            .filter_map(|p| {
                let i = paths.iter().position(|x| *x == p.path)?;
                // Liste de textes dans les données (sinon : pas une liste d'identifiants).
                let list = d.values.get(i)?.as_ref()?.as_array()?.clone();
                let slugs: Option<Vec<String>> = list
                    .iter()
                    .map(|v| v.as_str().map(str::to_string))
                    .collect();
                Some((p, slugs?, d.labels.get(i).cloned().flatten()))
            })
            .collect();
        if shown.is_empty() {
            return ().into_any();
        }
        let entries = StoredValue::new(d.entries.clone());
        view! {
            <h3 class="page-settings-section" title=ui.t("picks-help")>{ui.t("picks")}</h3>
            <div class="page-settings-seo picks">
                {shown.into_iter().map(|(p, slugs, label)| {
                    let path = StoredValue::new(p.path.clone());
                    let current = StoredValue::new(slugs.clone());
                    let write = move |v: Vec<String>| {
                        ed.apply(ui, data_command(&path.get_value(), Some(Value::Array(v.into_iter().map(Value::String).collect()))), false)
                    };
                    let name = label.unwrap_or_else(|| p.path.last().cloned().unwrap_or_default());
                    let count = slugs.len();
                    let cols = p.collections.clone();
                    view! {
                        <div class="pick" data-pick=p.path.join(".")>
                            <span class="label" title=p.path.join(".")>{name}</span>
                            <ul class="pick-list">
                                {slugs.into_iter().enumerate().map(|(i, slug)| {
                                    let found = entries.with_value(|e| title_of(e, &slug));
                                    let missing = found.is_none();
                                    let (t, c) = found.unwrap_or_else(|| (slug.clone(), String::new()));
                                    let shift = move |dlt: isize| {
                                        let mut all = current.get_value();
                                        let j = i as isize + dlt;
                                        if j >= 0 && (j as usize) < all.len() {
                                            all.swap(i, j as usize);
                                            write(all);
                                        }
                                    };
                                    view! {
                                        <li class="pick-item" class:is-missing=missing
                                            title=if missing { ui.now("picks-missing", &[("slug", slug.clone())]) } else { format!("{t} · {c}") }>
                                            <span class="pick-title">{t.clone()}</span>
                                            <button class="icon-btn icon-btn--small" title=ui.t("layers-up") aria-label=ui.t("layers-up")
                                                disabled=i == 0 on:click=move |_| shift(-1)>{icons::arrow_up()}</button>
                                            <button class="icon-btn icon-btn--small" title=ui.t("layers-down") aria-label=ui.t("layers-down")
                                                disabled=i + 1 == count on:click=move |_| shift(1)>{icons::arrow_down()}</button>
                                            <button class="icon-btn icon-btn--small" title=ui.t("action-remove") aria-label=ui.t("action-remove")
                                                on:click=move |_| { let mut all = current.get_value(); all.remove(i); write(all); }>{icons::close()}</button>
                                        </li>
                                    }
                                }).collect_view()}
                            </ul>
                            <div class="select pick-add">
                                <select class="input" aria-label=ui.t("picks-add")
                                    on:change=move |e| {
                                        let v = event_target_value(&e);
                                        if !v.is_empty() {
                                            let mut all = current.get_value();
                                            all.push(v);
                                            write(all);
                                        }
                                    }>
                                    <option value="" selected=true>{ui.t("picks-add")}</option>
                                    {entries.with_value(|es| es.iter()
                                        .filter(|e| cols.contains(&e.collection) && !current.with_value(|c| c.contains(&e.slug)))
                                        .map(|e| view! { <option value=e.slug.clone()>{format!("{} · {}", e.title, e.collection)}</option> })
                                        .collect_view())}
                                </select>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        }
        .into_any()
    }
}
