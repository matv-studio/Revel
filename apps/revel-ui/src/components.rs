//! Composants dans l'éditeur (3.6, D-34), comme ceux de Webflow et Framer : réglages d'une
//! utilisation (valeurs de ses propriétés, variante, ouvrir le composant, détacher), propriétés
//! du composant ouvert (ajouter, renommer, valeur par défaut, supprimer), variantes (survol,
//! appui, personnalisées) dans le canevas des composants, éléments du composant reliés à une
//! propriété (texte, image, lien, visibilité), création d'un composant à partir de la sélection.

use crate::editor::{Ed, Selection};
use crate::inspector::Field;
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::canvas::{literal, quote};
use revel_edit::{Branch, Command, Owner, Parent, Target};
use revel_model::{Component, Node, PropDef, PropKind, Variant, VariantKind};
use serde_json::Value;
use std::collections::BTreeMap;

/// Types proposés à la création. « Contenu » (collection fixée dans le composant) ne l'est plus
/// (D-191) : une utilisation relie chaque propriété à un champ du contenu qu'elle choisit.
pub(crate) const KINDS: [PropKind; 9] = [
    PropKind::Text,
    PropKind::Number,
    PropKind::Choice,
    PropKind::Image,
    PropKind::Icon,
    PropKind::Link,
    PropKind::Color,
    PropKind::Switch,
    PropKind::List,
];

pub(crate) fn kind_key(k: PropKind) -> &'static str {
    match k {
        PropKind::Text => "prop-kind-text",
        PropKind::Image => "prop-kind-image",
        PropKind::Link => "prop-kind-link",
        PropKind::Color => "prop-kind-color",
        PropKind::Switch => "prop-kind-switch",
        PropKind::List => "prop-kind-list",
        PropKind::Entry => "prop-kind-entry",
        PropKind::Number => "prop-kind-number",
        PropKind::Choice => "prop-kind-choice",
        PropKind::Icon => "prop-kind-icon",
    }
}

pub(crate) fn kind_icon(k: PropKind) -> AnyView {
    match k {
        PropKind::Text => icons::text(),
        PropKind::Image => icons::image(),
        PropKind::Link => icons::link(),
        PropKind::Color => icons::palette(),
        PropKind::Switch => icons::eye(),
        PropKind::List => icons::list(),
        PropKind::Entry => icons::database(),
        PropKind::Number => icons::hash(),
        PropKind::Choice => icons::list_ordered(),
        PropKind::Icon => icons::sparkle(),
    }
}

pub(crate) fn kind_icon_fn(k: PropKind) -> fn() -> AnyView {
    match k {
        PropKind::Text => icons::text,
        PropKind::Image => icons::image,
        PropKind::Link => icons::link,
        PropKind::Color => icons::palette,
        PropKind::Switch => icons::eye,
        PropKind::List => icons::list,
        PropKind::Entry => icons::database,
        PropKind::Number => icons::hash,
        PropKind::Choice => icons::list_ordered,
        PropKind::Icon => icons::sparkle,
    }
}

/// Début du nom d'une nouvelle propriété (identifiant dans les expressions).
fn base_name(k: PropKind) -> &'static str {
    match k {
        PropKind::Text => "texte",
        PropKind::Image => "image",
        PropKind::Link => "lien",
        PropKind::Color => "couleur",
        PropKind::Switch => "visible",
        PropKind::List => "liste",
        PropKind::Entry => "contenu",
        PropKind::Number => "nombre",
        PropKind::Choice => "choix",
        PropKind::Icon => "icone",
    }
}

/// Valeur par défaut en texte.
pub(crate) fn default_text(p: &PropDef) -> String {
    match &p.default {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string(),
    }
}

pub(crate) fn default_bool(p: &PropDef) -> bool {
    match &p.default {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s == "true",
        _ => false,
    }
}

/// Nombre écrit (`3`, `2,5`) -> valeur ; autre chose : `None`.
pub(crate) fn num_value(text: &str) -> Option<Value> {
    let t = text.trim().replace(',', ".");
    if let Ok(i) = t.parse::<i64>() {
        return Some(Value::from(i));
    }
    t.parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map(Value::Number)
}

/// Choix d'une icône de Revel (variable « Icône », D-196) : l'icône en place, ouverte sur la
/// grille des icônes par famille.
fn icon_choice(
    ui: Ui,
    current: String,
    label: String,
    on: impl Fn(String) + Clone + Send + Sync + 'static,
) -> AnyView {
    let path = revel_edit::icons::icon(&current).unwrap_or_default();
    view! {
        <details class="prop-icon" data-icon=current.clone()>
            <summary class="input prop-icon-current" aria-label=label title=current.clone()>
                <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2"
                    stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" inner_html=path></svg>
                <span>{current.clone()}</span>
            </summary>
            {revel_edit::icons::GROUPS.iter().map(|(k, names)| {
                let on = on.clone();
                let current = current.clone();
                view! {
                    <p class="prop-icon-group">{ui.t(k)}</p>
                    <div class="icon-grid" role="listbox" aria-label=ui.t(k)>
                        {names.iter().filter_map(|n| revel_edit::icons::ICONS.iter().find(|(x, _)| x == n)).map(|(name, path)| {
                            let on = on.clone();
                            let sel = current == *name;
                            view! {
                                <button type="button" class="icon-choice" class:is-on=sel title=*name aria-label=*name data-icon=*name
                                    on:click=move |_| on(name.to_string())>
                                    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"
                                        stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" inner_html=*path></svg>
                                </button>
                            }
                        }).collect_view()}
                    </div>
                }
            }).collect_view()}
        </details>
    }
    .into_any()
}

/// Couleurs du site proposées à une propriété « Couleur » (D-184) : (nom sans `--`, libellé,
/// valeur claire).
pub(crate) fn site_color_names(ed: Ed) -> Vec<(String, String, String)> {
    ed.doc.with_untracked(|d| {
        d.as_ref()
            .map(|d| {
                crate::sitecolors::colors(d)
                    .into_iter()
                    .map(|(var, light, _)| {
                        (
                            var.trim_start_matches("--").to_string(),
                            crate::names::humanize(&var),
                            light,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Menu d'une couleur du site, avec sa pastille ; `empty` : libellé du choix « aucune » (valeur
/// par défaut du composant, ou pas de couleur).
fn color_select(
    ed: Ed,
    current: String,
    empty: String,
    label: String,
    on: impl Fn(Option<String>) + 'static,
) -> AnyView {
    let colors = site_color_names(ed);
    let swatch = colors
        .iter()
        .find(|(n, _, _)| *n == current)
        .map(|(_, _, v)| v.clone())
        .unwrap_or_else(|| "transparent".into());
    // Valeur écrite à la main (ancien site) : gardée dans la liste.
    let unknown = (!current.is_empty() && !colors.iter().any(|(n, _, _)| *n == current))
        .then(|| current.clone());
    view! {
        <span class="prop-color">
            <span class="prop-color-swatch" style=format!("background:{swatch}")></span>
            <span class="select">
                <select class="input" aria-label=label
                    on:change=move |e| {
                        let v = event_target_value(&e);
                        on((!v.is_empty()).then_some(v));
                    }>
                    <option value="" selected=current.is_empty()>{empty}</option>
                    {unknown.map(|u| view! { <option value=u.clone() selected=true>{u.clone()}</option> })}
                    {colors.into_iter().map(|(n, l, _)| {
                        let on = n == current;
                        view! { <option value=n selected=on>{l}</option> }
                    }).collect_view()}
                </select>
            </span>
        </span>
    }
    .into_any()
}

/// Commande qui fait porter à l'élément racine du composant la couleur choisie par chaque
/// utilisation : `style="--<propriété>: var(--{{ <propriété> }})"`, lue par les styles du
/// composant (`var(--<propriété>)`, proposée par le sélecteur de couleur).
fn carry_color(c: &Component, prop: &str) -> Option<Command> {
    let root = c.body.iter().find_map(|n| match n {
        Node::Element { id, attrs, .. } => Some((id.clone(), attrs)),
        _ => None,
    })?;
    let decl = format!("--{prop}: var(--{{{{ {prop} }}}})");
    let old = root
        .1
        .iter()
        .find(|a| a.name == "style")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    if old.contains(&format!("--{prop}:")) {
        return None;
    }
    let value = match old.trim().trim_end_matches(';') {
        "" => decl,
        o => format!("{o}; {decl}"),
    };
    Some(Command::SetAttr {
        at: revel_edit::Target {
            owner: Owner::Component(c.id.clone()),
            node: root.0?,
        },
        name: "style".into(),
        value: Some(value),
    })
}

fn component(ed: Ed, id: &str) -> Option<Component> {
    ed.doc
        .with_untracked(|d| d.as_ref()?.component(id).cloned())
}

/// Propriétaire de propriétés : composant (valeur choisie à chaque utilisation) ou gabarit
/// (valeur choisie par chaque page qui l'utilise, dans ses réglages).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Holder {
    Component(String),
    Layout(String),
}

impl Holder {
    /// Propriétaire du nœud `at` : son composant ou son gabarit.
    pub fn of(at: &Target) -> Option<Holder> {
        match &at.owner {
            Owner::Component(c) => Some(Holder::Component(c.clone())),
            Owner::Layout(l) => Some(Holder::Layout(l.clone())),
            Owner::Page(_) => None,
        }
    }
    /// Propriétés que le propriétaire déclare.
    pub(crate) fn own(&self, site: &revel_model::Site) -> Option<Vec<PropDef>> {
        match self {
            Holder::Component(id) => site.component(id).map(|c| c.props.clone()),
            Holder::Layout(id) => site
                .layouts
                .iter()
                .find(|l| &l.id == id)
                .map(|l| l.fields.clone()),
        }
    }
    /// Propriétés lisibles dans le propriétaire : les siennes ; pour un gabarit, aussi celles
    /// des gabarits qui le contiennent.
    pub(crate) fn visible(&self, site: &revel_model::Site) -> Option<Vec<PropDef>> {
        match self {
            Holder::Component(_) => self.own(site),
            Holder::Layout(id) => {
                self.own(site)?;
                Some(revel_edit::editable::layout_fields(site, Some(id)))
            }
        }
    }
    pub(crate) fn set(&self, props: Vec<PropDef>) -> Command {
        match self {
            Holder::Component(id) => Command::SetComponentProps {
                id: id.clone(),
                props,
            },
            Holder::Layout(id) => Command::SetLayoutFields {
                id: id.clone(),
                fields: props,
            },
        }
    }
    /// Retrait d'une propriété (valeur par défaut écrite là où elle servait).
    pub(crate) fn remove(&self, site: &revel_model::Site, name: &str) -> Option<Command> {
        match self {
            Holder::Component(id) => revel_edit::make::remove_prop(site, id, name).ok(),
            Holder::Layout(id) => revel_edit::make::remove_layout_field(site, id, name).ok(),
        }
    }
}

/// Premier nœud repéré (avec identifiant) d'une liste.
fn first_id(nodes: &[Node]) -> Option<(String, String)> {
    let mut found = None;
    revel_model::ids::walk(nodes, &mut |n| {
        if found.is_none()
            && let Some(id) = revel_model::ids::node_id(n)
        {
            let tag = match n {
                Node::Element { tag, .. } => tag.clone(),
                Node::Image(_) => "img".into(),
                _ => String::new(),
            };
            found = Some((id.to_string(), tag));
        }
    });
    found
}

/// Sélectionne un nœud et le montre dans le canevas.
fn select(ed: Ed, id: String, tag: String, label: String) {
    ed.selected.set(Some(Selection {
        id: id.clone(),
        tag,
        label,
    }));
    ed.select_and_reveal(&id);
}

/// Ouvre le composant d'une utilisation dans le canevas des composants : son premier élément est
/// sélectionné.
pub fn open_component(ed: Ed, use_at: &Target) {
    let found = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let Node::Use { component, .. } = revel_edit::address::node(d, use_at).ok()? else {
            return None;
        };
        Some((component.clone(), first_id(&d.component(component)?.body)))
    });
    if let Some((c, first)) = found {
        ed.open_comp(&c, first);
    }
}

/// Utilisation du composant `id` (la première trouvée), avec son propriétaire.
fn find_use(ed: Ed, id: &str) -> Option<String> {
    ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let mut found = None;
        for (_, _, body) in d.owners() {
            revel_model::ids::walk(body, &mut |n| {
                if found.is_none()
                    && let Node::Use {
                        id: Some(u),
                        component,
                        ..
                    } = n
                    && component == id
                {
                    found = Some(u.clone());
                }
            });
        }
        found
    })
}

/// « Nouveau composant » (menu du panneau, D-196) : composant vide nommé, ouvert pour être rempli.
pub fn create_empty(ui: Ui, ed: Ed) {
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("components-new-empty", &[]),
        ui.now("component-make-name", &[]),
        String::new(),
        ui.now("component-make-confirm", &[]),
        move |name| {
            let Some(c) = ed.doc.with_untracked(|d| {
                d.as_ref()
                    .map(|d| revel_edit::make::empty_component(d, &name))
            }) else {
                return;
            };
            let id = c.id.clone();
            ed.apply(
                ui,
                Command::AddComponent {
                    component: c,
                    index: None,
                },
                false,
            );
            // Ouvert dès que le document porte le composant (la commande est appliquée par le cœur).
            leptos::task::spawn_local(async move {
                for _ in 0..40 {
                    let there = ed
                        .doc
                        .with_untracked(|d| d.as_ref().is_some_and(|d| d.component(&id).is_some()));
                    if there {
                        edit_component(ui, ed, &id);
                        return;
                    }
                    crate::sleep_ms(50).await;
                }
            });
        },
    )));
}

/// « Coller » un composant copié (menu du panneau, D-196) : sa copie, nouveaux identifiants.
pub fn paste_copy(ui: Ui, ed: Ed, id: &str) {
    let word = ui.now("pages-copy-word", &[]);
    let copy = ed
        .doc
        .with_untracked(|d| revel_edit::make::duplicate_component(d.as_ref()?, id, &word).ok());
    if let Some(c) = copy {
        ed.apply(
            ui,
            Command::AddComponent {
                component: c,
                index: None,
            },
            false,
        );
    }
}

/// Crée un composant à partir de l'élément sélectionné (nom demandé), puis sélectionne son
/// utilisation.
pub fn make_from_selection(ui: Ui, ed: Ed) {
    let Some(at) = untrack(|| ed.selected_target()) else {
        ui.fail(revel_protocol::Failure::new("component-make-none"));
        return;
    };
    let suggestion = ed
        .doc
        .with_untracked(|d| {
            let n = revel_edit::address::node(d.as_ref()?, &at).ok()?;
            Some(match n {
                Node::Element { tag, attrs, .. } => attrs
                    .iter()
                    .find(|a| a.name == "class")
                    .and_then(|a| a.value.as_deref())
                    .and_then(|c| c.split_whitespace().next())
                    .map(|c| c.replace(['_', '-'], " "))
                    .unwrap_or_else(|| tag.clone()),
                _ => String::new(),
            })
        })
        .unwrap_or_default();
    let suggestion = crate::site::capitalize(suggestion.trim());
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("component-make", &[]),
        ui.now("component-make-name", &[]),
        suggestion,
        ui.now("component-make-confirm", &[]),
        move |name| {
            let Some(id) = ed.doc.with_untracked(|d| {
                d.as_ref()
                    .map(|d| revel_edit::make::free_component_id(d, &name))
            }) else {
                return;
            };
            let mut make = Command::MakeComponent {
                at: at.clone(),
                id: id.clone(),
                name: Some(name.trim().to_string()),
                use_id: None,
            };
            // Carte d'une liste de collection (D-193) : ce qu'elle lit du contenu devient des
            // propriétés, reliées aux champs dans la liste ; une seule étape d'annulation.
            let list = ed.doc.with_untracked(|d| {
                crate::blocks::enclosing_list(d.as_ref()?, &at).map(|(q, item)| (q.key(), item))
            });
            let Some((collection, var)) = list else {
                ed.apply(ui, make, false);
                if let Some(u) = find_use(ed, &id) {
                    select(ed, u, String::new(), name.clone());
                }
                return;
            };
            let path = ed.path.get_value();
            leptos::task::spawn_local(async move {
                let schema = crate::api::cms_collections(&path)
                    .await
                    .map(|c| schema_of(&c))
                    .unwrap_or_default();
                let lifted = ed.doc.with_untracked(|d| {
                    let mut after = d.as_ref()?.clone();
                    revel_edit::apply(&mut after, &make).ok()?;
                    // Même utilisation à l'application réelle : son identifiant est fixé.
                    let mut made = None;
                    for body in after
                        .pages
                        .iter()
                        .map(|p| &p.body)
                        .chain(after.layouts.iter().map(|l| &l.body))
                    {
                        revel_model::ids::walk(body, &mut |n| {
                            if let Node::Use {
                                id: Some(u),
                                component,
                                ..
                            } = n
                                && *component == id
                            {
                                made = Some(u.clone());
                            }
                        });
                    }
                    if let Command::MakeComponent { use_id, .. } = &mut make {
                        *use_id = made;
                    }
                    revel_edit::entryprops::lift_list_reads(
                        &after,
                        &id,
                        &var,
                        &collection,
                        &schema,
                        &ui.now("prop-page", &[]),
                    )
                });
                let n = lifted.as_ref().map(|l| l.1).unwrap_or(0);
                let cmd = match lifted {
                    Some((mut commands, _)) => {
                        commands.insert(0, make);
                        Command::Batch { commands }
                    }
                    None => make,
                };
                ed.apply(ui, cmd, false);
                if let Some(u) = find_use(ed, &id) {
                    select(ed, u, String::new(), name.clone());
                }
                if n > 0 {
                    ui.fail(
                        revel_protocol::Failure::new("info-undo-component-fields")
                            .arg("name", name.trim())
                            .arg("n", n.to_string()),
                    );
                }
            });
        },
    )));
}

/// Détache une utilisation : copie modifiable séparément.
pub fn detach(ui: Ui, ed: Ed, at: Target) {
    ed.apply(ui, Command::Detach { at }, false);
}

// ------------------------------------------------------------------ utilisation sélectionnée

/// Réglages d'une utilisation de composant : nom, ouvrir, détacher, valeurs des propriétés.
pub fn instance_settings(
    ui: Ui,
    ed: Ed,
    at: Target,
    component_id: String,
    args: BTreeMap<String, String>,
) -> AnyView {
    let Some(c) = component(ed, &component_id) else {
        return view! { <p class="muted">{ui.tx("component-missing", &[("name", component_id)])}</p> }
            .into_any();
    };
    let at = StoredValue::new(at);
    let set = move |name: String, value: Option<String>| {
        ed.apply(
            ui,
            Command::SetUseArg {
                at: at.get_value(),
                name,
                value,
            },
            false,
        )
    };
    // Source du contenu (D-191) : la liste de collection qui entoure l'utilisation ; sinon la
    // collection et le contenu choisis ici (lus dans les valeurs reliées) ; sinon aucune.
    let list = ed.doc.with_untracked(|d| {
        crate::blocks::enclosing_list(d.as_ref()?, &at.get_value()).map(|(q, item)| (q.key(), item))
    });
    let list_id = ed
        .doc
        .with_untracked(|d| crate::blocks::enclosing_list_id(d.as_ref()?, &at.get_value()));
    let list_id = StoredValue::new(list_id);
    let cols = crate::blocks::collections(ed);
    // Collections déjà lues : montrées tout de suite (le panneau est refait à chaque réglage, la
    // relecture arrive un instant après).
    let cached = crate::textfield::cached_cols();
    if !cached.is_empty() {
        cols.set(cached);
    }
    let use_id = at.with_value(|a| a.node.clone());
    // Une variable de cette utilisation lit un champ du contenu (`….data.x`, adresse de la page).
    let reads_content = args
        .values()
        .any(|a| a.contains(".data.") || a.trim().ends_with(".url"));
    let from_args = args.values().find_map(|a| {
        let p = arg_prefix(a)?;
        let (c, s) = parse_prefix(&p)?;
        Some((c, s, p))
    });
    let picked = RwSignal::new(if list.is_some() {
        None
    } else {
        from_args
            .clone()
            .map(|(c, s, _)| (c, s))
            .or_else(|| PICKED.with(|k| k.borrow().get(&use_id).cloned()))
    });
    // Préfixe du contenu choisi, tel qu'il est écrit dans les valeurs.
    let written_prefix = StoredValue::new(from_args.map(|(_, _, p)| p));
    let prefix_of = move |pick: &(String, Option<String>)| {
        written_prefix
            .get_value()
            .unwrap_or_else(|| entry_prefix(&pick.0, pick.1.as_deref()))
    };
    let list = StoredValue::new(list);
    let use_id = StoredValue::new(use_id);
    // Contenu montré en aperçu : celui de l'utilisation sélectionnée (carte d'une liste), sinon
    // le contenu choisi, sinon le premier de la collection.
    let sel_entries = crate::textfield::selection_entries(ed);
    let sample: Sample = Signal::derive(move || {
        let (col, slug) = match (list.get_value(), picked.get()) {
            (Some((c, _)), _) => (c, None),
            (None, Some((c, s))) => (c, s),
            _ => return None,
        };
        let here = sel_entries.get();
        cols.with(|cs| {
            let c = cs.iter().find(|c| c.id == col)?;
            let e = here
                .iter()
                .filter(|(folder, _)| *folder == c.folder)
                .find_map(|(_, file)| c.entries.iter().find(|e| e.file == *file))
                .or_else(|| match &slug {
                    Some(s) => c.entries.iter().find(|e| entry_slug_of(e) == *s),
                    None => c.entries.first(),
                })?;
            Some((
                c.folder.clone(),
                e.file.clone(),
                e.data.clone(),
                e.url.clone(),
            ))
        })
    });
    // Propriétés de ce qui contient l'utilisation (composant ouvert, gabarit) : une valeur peut
    // en reprendre une (`titre`, `not masquer`).
    let parents: Vec<PropDef> = ed
        .doc
        .with_untracked(|d| {
            let d = d.as_ref()?;
            Holder::of(&at.get_value())?.visible(d)
        })
        .unwrap_or_default();
    let lang = ed.lang();
    // Propriétés qui remplacent un champ du contenu : réglées dans « Champs CMS » (D-179).
    let overrides = revel_edit::overrides::override_props(&c);
    let shown_props: Vec<PropDef> = c
        .props
        .iter()
        .filter(|p| !overrides.contains(&p.name))
        .cloned()
        .collect();
    let has_props = !shown_props.is_empty();
    let shown_props = StoredValue::new(shown_props);
    let args = StoredValue::new(args);
    let parents = StoredValue::new(parents);
    let rows = move || {
        // Champs lisibles : ceux de la liste, sinon ceux du contenu choisi.
        let bind: Option<ListBind> = list
            .get_value()
            .or_else(|| picked.get().map(|p| (p.0.clone(), prefix_of(&p))))
            .map(|b| (b, cols, sample));
        shown_props
            .get_value()
            .into_iter()
            .map(|p| {
            let arg = args.with_value(|a| a.get(&p.name).cloned());
            // Autre langue : texte de la propriété traduit (valeur de l'utilisation, sinon
            // valeur par défaut).
            if let Some(code) = lang.clone()
                && p.kind == PropKind::Text
                && let Some(src) = match &arg {
                    Some(a) => literal(a),
                    None => p.default.as_ref().and_then(|v| v.as_str()).map(str::to_string),
                }
                .filter(|t| revel_edit::locale::translatable(t))
            {
                let name = p.name.clone();
                return view! {
                    <div class="prop-row" data-prop=p.name.clone()>
                        <Field label=p.label().to_string() value=ed.translation(&code, &src).unwrap_or_default()
                            placeholder=src.clone()
                            on_commit=move |v: String| ed.apply(ui, Command::Translate {
                                locale: code.clone(),
                                place: revel_edit::locale::TextPlace::Arg { at: at.get_value(), name: name.clone() },
                                text: v,
                            }, false)/>
                    </div>
                }
                .into_any();
            }
            let parent = arg.as_deref().and_then(|a| {
                let a = a.trim();
                let (neg, name) = match a.strip_prefix("not ") {
                    Some(x) => (true, x.trim()),
                    None => (false, a),
                };
                parents.with_value(|ps| {
                    ps.iter()
                        .find(|q| q.name == name && (!neg || q.kind == PropKind::Switch))
                        .map(|q| (q.clone(), neg))
                })
            });
            prop_value_row(ui, ed, p.clone(), arg, set, bind.clone(), parent)
        })
        .collect_view()
    };
    // Collection choisie (hors d'une liste) : chaque propriété libre ou déjà reliée prend le
    // champ de même nom ; « Aucune » rend les valeurs par défaut.
    let set_collection = move |v: String| {
        let new = (!v.is_empty()).then_some(v);
        let old = picked.get_untracked().map(|p| prefix_of(&p));
        let np = new.as_deref().map(|c| entry_prefix(c, None));
        let mut commands = Vec::new();
        for p in shown_props
            .get_value()
            .iter()
            .filter(|p| p.kind != PropKind::Color)
        {
            let arg = args.with_value(|a| a.get(&p.name).cloned());
            let was = old
                .as_deref()
                .is_some_and(|o| arg.as_deref().is_some_and(|a| a.contains(o)));
            if !(arg.is_none() || was) {
                continue;
            }
            let target = new.as_deref().zip(np.as_deref()).and_then(|(c, np)| {
                cols.with_untracked(|cs| {
                    let c = cs.iter().find(|x| x.id == c)?;
                    let label = p.label().to_lowercase();
                    c.fields
                        .iter()
                        .filter(|f| field_fits(p.kind, f))
                        .find(|f| {
                            f.name == p.name
                                || f.label
                                    .as_deref()
                                    .is_some_and(|l| l.to_lowercase() == label)
                        })
                        .map(|f| format!("{np}.data.{}", f.name))
                        .or_else(|| {
                            (p.kind == PropKind::Link && p.name.starts_with("lien_page"))
                                .then(|| format!("{np}.url"))
                        })
                })
            });
            if target != arg {
                commands.push(Command::SetUseArg {
                    at: at.get_value(),
                    name: p.name.clone(),
                    value: target,
                });
            }
        }
        PICKED.with(|k| {
            let mut k = k.borrow_mut();
            match &new {
                Some(c) => k.insert(use_id.get_value(), (c.clone(), None)),
                None => k.remove(&use_id.get_value()),
            };
        });
        written_prefix.set_value(None);
        picked.set(new.map(|c| (c, None)));
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    // Contenu choisi : les valeurs reliées lisent le nouveau contenu.
    let set_entry = move |v: String| {
        let Some(pick) = picked.get_untracked() else {
            return;
        };
        let slug = (!v.is_empty()).then_some(v);
        let old = prefix_of(&pick);
        let new = entry_prefix(&pick.0, slug.as_deref());
        let commands: Vec<Command> = args.with_value(|a| {
            a.iter()
                .filter(|(_, x)| x.contains(old.as_str()))
                .map(|(n, x)| Command::SetUseArg {
                    at: at.get_value(),
                    name: n.clone(),
                    value: Some(x.replace(old.as_str(), &new)),
                })
                .collect()
        });
        PICKED.with(|k| {
            k.borrow_mut()
                .insert(use_id.get_value(), (pick.0.clone(), slug.clone()))
        });
        written_prefix.set_value(None);
        picked.set(Some((pick.0, slug)));
        if !commands.is_empty() {
            ed.apply(ui, Command::Batch { commands }, false);
        }
    };
    let col_label = move |id: &str| {
        cols.with(|cs| {
            cs.iter()
                .find(|c| c.id == id)
                .map(crate::blocks::label_of)
                .unwrap_or_else(|| crate::site::capitalize(id))
        })
    };
    let source = move || {
        if let Some((col, _)) = list.get_value() {
            let id = list_id.get_value();
            return view! {
                <div class="view-line content-source" data-source="list" title=ui.t("content-source-list-help")>
                    <span class="prop-label">{ui.t("content-source")}</span>
                    <span class="prop-chip prop-chip--field prop-chip--plain">{icons::repeat()}<span>{move || ui.tx("layers-each", &[("list", col_label(&col))])}</span></span>
                    {id.map(|id| view! {
                        <button type="button" class="link-btn" data-action="source-list" title=ui.t("content-source-edit-help")
                            on:click=move |_| ed.select_and_reveal(&id)>{ui.t("content-source-edit")}</button>
                    })}
                </div>
            }
            .into_any();
        }
        let pick = picked.get();
        let cur = pick.as_ref().map(|p| p.0.clone()).unwrap_or_default();
        let slug = pick.as_ref().and_then(|p| p.1.clone()).unwrap_or_default();
        let cur2 = cur.clone();
        view! {
            <label class="view-line content-source" data-source="collection">
                <span class="prop-label">{ui.t("prop-entry-collection")}</span>
                <span class="select prop-field">
                    <select class="input content-collection" on:change=move |e| set_collection(event_target_value(&e))>
                        <option value="" prop:selected=cur.is_empty()>{ui.t("content-none")}</option>
                        {move || cols.with(|cs| cs.iter().filter(|c| !c.id.contains('+')).map(|c| {
                            let on = c.id == cur;
                            view! { <option value=c.id.clone() prop:selected=on>{crate::blocks::label_of(c)}</option> }
                        }).collect_view())}
                    </select>
                </span>
            </label>
            {pick.is_some().then(|| view! {
                <label class="view-line content-source" data-source="entry">
                    <span class="prop-label">{ui.t("prop-entry-content")}</span>
                    <span class="select prop-field">
                        <select class="input content-entry" on:change=move |e| set_entry(event_target_value(&e))>
                            <option value="" prop:selected=slug.is_empty()>{ui.t("prop-entry-first")}</option>
                            {move || entries_of(cols, &cur2).into_iter().map(|(s, t)| {
                                let on = s == slug;
                                view! { <option value=s prop:selected=on>{t}</option> }
                            }).collect_view()}
                        </select>
                    </span>
                </label>
            })}
        }
        .into_any()
    };
    // Section « Contenu » seulement quand l'utilisation lit un champ du CMS (retour du 24/11/2026,
    // D-196) : un composant sans champ CMS ne montre que ses variables. Dans une liste, le bouton
    // de chaque variable relie un champ ; la section apparaît alors.
    // Une fois montrée pour une utilisation, elle reste le temps de la session (« Aucune » choisi
    // par erreur : la collection se choisit de nouveau).
    let shown_before = SHOWN_CONTENT.with(|k| {
        let mut k = k.borrow_mut();
        let id = use_id.get_value();
        if reads_content && !k.contains(&id) {
            k.push(id.clone());
        }
        k.contains(&id)
    });
    let content =
        move || shown_before && (list.with_value(Option::is_some) || cols.with(|c| !c.is_empty()));
    let open_cms = move || {
        (list.with_value(Option::is_some) || picked.with(Option::is_some))
            .then(|| sample.get())
            .flatten()
            .map(|(folder, file, _, _)| {
                view! {
                    <button type="button" class="btn btn--small cms-fields-open" data-action="cms-open-entry"
                        on:click=move |_| {
                            ui.goto_entry.set(Some((folder.clone(), file.clone())));
                            ui.tab(crate::Tab::Cms);
                        }>{icons::database()}<span>{ui.t("cms-open-entry")}</span></button>
                }
            })
    };
    // Variante choisie pour cette utilisation (variantes personnalisées du composant).
    let customs: Vec<Variant> = c
        .variants
        .iter()
        .filter(|v| v.kind == VariantKind::Custom)
        .cloned()
        .collect();
    let chosen = ed.doc.with_untracked(|d| {
        match revel_edit::address::node(d.as_ref()?, &at.get_value()).ok()? {
            Node::Use { variant, .. } => variant.clone(),
            _ => None,
        }
    });
    let variant_row = (!customs.is_empty()).then(|| {
        let chosen = chosen.clone().unwrap_or_default();
        view! {
            <label class="field instance-variant">
                <span class="label">{ui.t("variant-label")}</span>
                <span class="select">
                    <select class="input" on:change=move |e| {
                        let v = event_target_value(&e);
                        ed.apply(ui, Command::SetUseVariant {
                            at: at.get_value(),
                            variant: (!v.is_empty()).then_some(v),
                        }, false);
                    }>
                        <option value="" selected=chosen.is_empty()>{ui.t("variant-primary")}</option>
                        {customs.iter().map(|v| {
                            let on = chosen == v.id;
                            view! { <option value=v.id.clone() selected=on>{variant_label(ui, Some(v))}</option> }
                        }).collect_view()}
                    </select>
                </span>
            </label>
        }
    });
    // Emplacement du composant (D-118) : contenu du composant, ou propre à cette utilisation.
    let slot_row = revel_edit::make::slot_of(&c).is_some().then(|| {
        let own = ed.doc.with_untracked(|d| {
            match d.as_ref().and_then(|d| revel_edit::address::node(d, &at.get_value()).ok()) {
                Some(Node::Use { children, .. }) => !children.is_empty(),
                _ => false,
            }
        });
        let run = move |reset: bool| {
            let cmd = ed.doc.with_untracked(|d| {
                let d = d.as_ref()?;
                let at = at.get_value();
                if reset {
                    revel_edit::make::reset_slot(d, &at).ok()
                } else {
                    revel_edit::make::customize_slot(d, &at).ok()
                }
            });
            if let Some(cmd) = cmd {
                ed.apply(ui, cmd, false);
            }
        };
        view! {
            <div class="insp-group instance-slot" data-slot=if own { "own" } else { "default" }>
                <p class="panel-title" title=ui.t("slot-help")>{ui.t("slot-title")}</p>
                <div class="slot-row">
                    <span class="muted small">{ui.t(if own { "slot-own" } else { "slot-default" })}</span>
                    {if own {
                        view! {
                            <button class="btn btn--small" data-action="slot-reset" title=ui.t("slot-reset-help")
                                on:click=move |_| run(true)>{icons::undo()}<span>{ui.t("slot-reset")}</span></button>
                        }.into_any()
                    } else {
                        view! {
                            <button class="btn btn--small" data-action="slot-edit" title=ui.t("slot-edit-help")
                                on:click=move |_| run(false)>{icons::edit()}<span>{ui.t("slot-edit")}</span></button>
                        }.into_any()
                    }}
                </div>
            </div>
        }
    });
    view! {
        <div class="stack instance">
            {variant_row}
            {slot_row}
            {has_props.then(|| view! {
                <div class="insp-group instance-content" class:has-source=content>
                    {move || if content() {
                        view! {
                            <p class="panel-title tool-head"><span class="tool-icon">{icons::database()}</span><span>{ui.t("content-title")}</span></p>
                            {source}
                            <div class="content-sep"></div>
                        }.into_any()
                    } else {
                        view! { <p class="panel-title">{ui.t("component-props")}</p> }.into_any()
                    }}
                    {rows}
                    {open_cms}
                </div>
            })}
        </div>
    }
    .into_any()
}

/// Contenu lisible par une utilisation : (collection, variable de la liste ou contenu choisi),
/// collections du site (champs), contenu montré en aperçu.
type ListBind = (
    (String, String),
    RwSignal<Vec<revel_protocol::CmsCollection>>,
    Sample,
);

/// Valeur d'une propriété reliée à un champ du contenu (`item` : variable de la liste, ou contenu
/// choisi entre parenthèses) : champ, autre champ lu s'il est vide, écriture d'une date
/// (`(item.data.a or item.data.b) | date("%d.%m.%Y")`).
fn bound_field(arg: &str, item: &str) -> Option<(String, Option<String>, Option<String>)> {
    let prefix = format!("{item}.data.");
    let field = |e: &str| {
        e.trim()
            .strip_prefix(prefix.as_str())
            // Nom de champ : tout identifiant (`site`, `page` ne sont réservés qu'aux propriétés).
            .filter(|f| !f.is_empty() && f.chars().all(|c| c.is_alphanumeric() || c == '_'))
            .map(str::to_string)
    };
    let fields = |head: &str| -> Option<(String, Option<String>)> {
        let head = head.trim();
        if let Some(f) = field(head) {
            return Some((f, None));
        }
        let inner = head.strip_prefix('(').and_then(|h| h.strip_suffix(')'))?;
        let (x, y) = inner.split_once(" or ")?;
        Some((field(x)?, Some(field(y)?)))
    };
    let a = arg.trim();
    if let Some((f, fb)) = fields(a) {
        return Some((f, fb, None));
    }
    // Le préfixe peut contenir `|` (`by_slug`) : l'écriture de la date est après le dernier.
    let (h, t) = a.rsplit_once('|')?;
    let (f, fb) = fields(h)?;
    Some((f, fb, Some(revel_edit::dates::format_of(t)?)))
}

/// Contenu choisi par une utilisation hors d'une liste, tel qu'il s'écrit devant `.data.<champ>` :
/// celui de l'identifiant `slug`, sinon le premier de la collection.
fn entry_prefix(collection: &str, slug: Option<&str>) -> String {
    match slug.filter(|s| !s.is_empty()) {
        Some(s) => format!(
            "(collections.{collection} | by_slug({}))",
            Value::String(s.to_string())
        ),
        None => format!("((collections.{collection})[0])"),
    }
}

/// Contenu lu par une valeur (`(…).data.titre`, `(…).url`, avec repli ou date) : le texte entre
/// parenthèses, parenthèses comprises.
fn arg_prefix(arg: &str) -> Option<String> {
    let a = arg.trim();
    if !a.starts_with('(') {
        return None;
    }
    let mut depth = 0usize;
    let mut end = None;
    for (i, c) in a.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let rest = &a[end + 1..];
    if rest.starts_with(".data.") || rest == ".url" {
        return Some(a[..=end].to_string());
    }
    // Repli `((…).data.a or (…).data.b)` : le contenu est dedans.
    arg_prefix(&a[1..end])
}

/// Collection et identifiant du contenu d'un préfixe (inverse de [`entry_prefix`]).
fn parse_prefix(prefix: &str) -> Option<(String, Option<String>)> {
    let mut e = prefix.trim();
    if let Some(inner) = e.strip_prefix('(').and_then(|x| x.strip_suffix(')')) {
        e = inner.trim();
    }
    if let Some(head) = e.strip_suffix("[0]") {
        let c = head
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .strip_prefix("collections.")?;
        return c
            .chars()
            .all(|x| x.is_alphanumeric() || x == '_' || x == '-')
            .then(|| (c.to_string(), None));
    }
    revel_edit::collection::entry_slug(e).map(|(c, s)| (c, Some(s)))
}

thread_local! {
    /// Utilisations dont la section « Contenu » a été montrée dans la session (elles lisaient un
    /// champ du CMS).
    static SHOWN_CONTENT: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

thread_local! {
    /// Collection choisie pour une utilisation dont aucune propriété n'est encore reliée
    /// (identifiant de l'utilisation → collection, contenu) : gardée le temps de la séance.
    static PICKED: std::cell::RefCell<std::collections::HashMap<String, (String, Option<String>)>> = Default::default();
}

/// Identifiant d'un contenu : son champ `slug`, sinon le nom de son fichier.
fn entry_slug_of(e: &revel_protocol::CmsEntry) -> String {
    e.data
        .get("slug")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            e.file
                .rsplit_once('.')
                .map_or(e.file.as_str(), |(a, _)| a)
                .to_string()
        })
}

/// Contenu montré en aperçu : dossier, fichier, champs, adresse de sa page.
type Sample = Signal<Option<(String, String, Value, Option<String>)>>;

/// Valeur d'un champ, courte, pour l'aperçu d'une ligne.
fn preview_text(v: &Value) -> String {
    match v {
        Value::String(s) => s
            .rsplit_once('/')
            .filter(|_| s.starts_with('/') || s.starts_with("http"))
            .map_or(s.as_str(), |(_, f)| f)
            .to_string(),
        Value::Bool(true) => "✓".into(),
        Value::Bool(false) | Value::Null => "—".into(),
        Value::Array(a) => a.iter().map(preview_text).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

/// Champs d'une collection qu'une propriété de ce type peut lire.
fn field_fits(kind: PropKind, f: &revel_protocol::CmsField) -> bool {
    let k = f.kind.as_deref();
    match kind {
        PropKind::List => f.list,
        PropKind::Switch => !f.list && !matches!(k, Some("object" | "block")),
        _ => !f.list && !matches!(k, Some("object" | "block" | "boolean")),
    }
}

/// Expression d'une propriété reliée (inverse de [`bound_field`]).
fn bound_expr(item: &str, field: &str, fallback: Option<&str>, format: Option<&str>) -> String {
    let head = match fallback {
        Some(fb) => format!("({item}.data.{field} or {item}.data.{fb})"),
        None => format!("{item}.data.{field}"),
    };
    match format {
        Some(f) => format!("{head} | {f}"),
        None => head,
    }
}

/// Valeur d'une propriété pour une utilisation : vide, la valeur par défaut s'applique.
fn prop_value_row(
    ui: Ui,
    ed: Ed,
    p: PropDef,
    arg: Option<String>,
    set: impl Fn(String, Option<String>) + Copy + Send + Sync + 'static,
    list: Option<ListBind>,
    parent: Option<(PropDef, bool)>,
) -> AnyView {
    // Propriété « Contenu » (D-174) : collection du composant, puis contenu choisi pour cette
    // utilisation (ou celui de la liste qui l'entoure).
    if p.kind == PropKind::Entry {
        let cols = crate::blocks::collections(ed);
        let col = p.collection.clone().unwrap_or_default();
        let col_label = {
            let col = col.clone();
            move || {
                cols.with(|cs| {
                    cs.iter()
                        .find(|c| c.id == col)
                        .map(|c| {
                            c.label
                                .clone()
                                .unwrap_or_else(|| crate::site::capitalize(&c.id))
                        })
                        .unwrap_or_else(|| col.clone())
                })
            }
        };
        let chosen = arg
            .as_deref()
            .and_then(revel_edit::collection::entry_slug)
            .map(|(_, s)| s);
        // Liste de la même collection autour de l'utilisation : son contenu.
        let item = list
            .as_ref()
            .filter(|((c, _), _, _)| *c == col)
            .map(|((_, item), _, _)| item.clone());
        let from_list = item.is_some() && arg.as_deref().map(str::trim) == item.as_deref();
        let current = if from_list {
            "@list".to_string()
        } else {
            chosen.map(|s| format!("slug:{s}")).unwrap_or_default()
        };
        let pname = StoredValue::new(p.name.clone());
        let def = StoredValue::new(p.clone());
        let item = StoredValue::new(item);
        let (c1, c2) = (current.clone(), col.clone());
        return view! {
            <div class="prop-entry" data-prop=p.name.clone()>
                <div class="style-row">
                    <span class="style-label"><span class="style-label-text">{ui.t("prop-entry-collection")}</span></span>
                    <div class="style-control"><span class="prop-entry-name" title=ui.t("prop-entry-collection-fixed")>{icons::database()}<span>{col_label}</span></span></div>
                </div>
                <div class="style-row">
                    <span class="style-label" title=ui.t("prop-entry-content-help")><span class="style-label-text">{p.label().to_string()}</span></span>
                    <div class="style-control"><div class="select">
                        // Carte d'une liste de collection : la liste fournit le contenu de chaque
                        // carte ; rien à choisir ici (D-181).
                        <select class="input prop-entry-select" aria-label=p.label().to_string()
                            prop:disabled=from_list
                            title=if from_list { ui.tx("prop-entry-list-help", &[]) } else { String::new() }
                            on:change=move |e| {
                                let v = event_target_value(&e);
                                let expr = if v == "@list" {
                                    item.get_value()
                                } else {
                                    v.strip_prefix("slug:").and_then(|s| def.with_value(|d| d.entry_expr(Some(s))))
                                };
                                set(pname.get_value(), expr);
                            }>
                            <option value="" selected=c1.is_empty()>{ui.t("prop-entry-first")}</option>
                            {item.with_value(Option::is_some).then(|| view! {
                                <option value="@list" selected=c1 == "@list">{ui.t("prop-entry-list")}</option>
                            })}
                            {move || entries_of(cols, &c2).into_iter().map(|(s, t)| {
                                let v = format!("slug:{s}");
                                let on = v == current;
                                view! { <option value=v selected=on>{t}</option> }
                            }).collect_view()}
                        </select>
                    </div></div>
                </div>
            </div>
        }
        .into_any();
    }
    let name = StoredValue::new(p.name.clone());
    let label = p.label().to_string();
    let overridden = arg.is_some();
    // Reprend une propriété du composant ouvert ou du gabarit (inversée pour `not x`).
    if let Some((q, neg)) = parent {
        let shown = if neg {
            ui.now(
                "component-prop-inverted",
                &[("label", q.label().to_string())],
            )
        } else {
            q.label().to_string()
        };
        let tip = format!("{{{{ {} }}}}", arg.clone().unwrap_or_default());
        return view! {
            <div class="view-line prop-row" data-prop=p.name.clone()>
                <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                <span class="prop-chip prop-chip--parent" title=tip>
                    {icons::prop_dot()}<span>{shown}</span>
                    <button class="chip-x" title=ui.t("component-unbind") aria-label=ui.t("component-unbind")
                        on:click=move |_| set(name.get_value(), None)>{icons::close()}</button>
                </span>
            </div>
        }
        .into_any();
    }
    // Relié au contenu (D-191) : un champ (avec repli sur un autre et écriture d'une date) ou
    // l'adresse de sa page. Un menu choisit le champ ; la valeur lue par l'utilisation
    // sélectionnée est montrée à côté.
    enum Bound {
        Field(String, Option<String>, Option<String>),
        Url,
    }
    let bound = list.as_ref().and_then(|((_, item), _, _)| {
        let a = arg.as_deref()?;
        if a.trim() == format!("{item}.url") {
            return (p.kind != PropKind::List).then_some(Bound::Url);
        }
        bound_field(a, item).map(|(f, fb, fmt)| Bound::Field(f, fb, fmt))
    });
    if let (Some(b), Some(((collection, item), cols, sample))) = (bound, list.clone()) {
        let kind = p.kind;
        let (field, fallback, format) = match &b {
            Bound::Field(f, fb, fmt) => (Some(f.clone()), fb.clone(), fmt.clone()),
            Bound::Url => (None, None, None),
        };
        let current = match &b {
            Bound::Url => "@url".to_string(),
            Bound::Field(_, Some(_), _) => "@keep".to_string(),
            Bound::Field(f, None, _) => format!("f:{f}"),
        };
        let col = StoredValue::new(collection);
        // Libellé d'un champ et sa nature (date : écriture choisie).
        let info = move |f: &str| {
            cols.with(|cs| {
                cs.iter()
                    .find(|c| col.with_value(|id| c.id == *id))
                    .and_then(|c| c.fields.iter().find(|x| x.name == f))
                    .map(|x| {
                        (
                            x.label
                                .clone()
                                .unwrap_or_else(|| revel_edit::overrides::field_label(&x.name)),
                            x.kind.clone(),
                        )
                    })
            })
            .unwrap_or_else(|| (revel_edit::overrides::field_label(f), None))
        };
        let shown = match (&field, &fallback) {
            (Some(f), Some(fb)) => Some(ui.now(
                "component-field-fallback",
                &[("field", info(f).0), ("other", info(fb).0)],
            )),
            _ => None,
        };
        let field = StoredValue::new(field);
        let date = Signal::derive(move || {
            format.is_some()
                || field.with_value(|f| {
                    f.as_deref().is_some_and(|f| {
                        info(f).1.as_deref().is_some_and(|k| k.starts_with("date"))
                    })
                })
        });
        let cur_fmt = StoredValue::new(
            match &b {
                Bound::Field(_, _, fmt) => fmt.clone(),
                Bound::Url => None,
            }
            .unwrap_or_default(),
        );
        let item = StoredValue::new(item);
        let fallback = StoredValue::new(fallback);
        // Valeur lue par l'utilisation montrée.
        let read = move || {
            {
                sample.with(|e| {
                    let (_, _, data, url) = e.as_ref()?;
                    match field.get_value() {
                        Some(f) => data
                            .get(&f)
                            .filter(|v| !v.is_null() && v.as_str() != Some(""))
                            .or_else(|| fallback.with_value(|fb| data.get(fb.as_deref()?)))
                            .cloned(),
                        None => url.clone().map(Value::String),
                    }
                })
            }
        };
        let options = move || {
            cols.with(|cs| {
                cs.iter()
                    .find(|c| col.with_value(|id| c.id == *id))
                    .map(|c| {
                        c.fields
                            .iter()
                            .filter(|f| {
                                field_fits(kind, f)
                                    || field.with_value(|x| x.as_deref() == Some(f.name.as_str()))
                            })
                            .map(|f| {
                                (
                                    f.name.clone(),
                                    f.label.clone().unwrap_or_else(|| {
                                        revel_edit::overrides::field_label(&f.name)
                                    }),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
        };
        let current = StoredValue::new(current);
        let lang = ed.lang().unwrap_or_else(|| {
            ed.doc
                .with_untracked(|d| d.as_ref().map(|d| d.default_locale.clone()))
                .unwrap_or_else(|| ui.lang())
        });
        let label2 = label.clone();
        return view! {
            <div class="prop-row prop-row--bound" data-prop=p.name.clone()>
                <div class="view-line">
                    <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                    <span class="select prop-field" class:has-preview=move || read().is_some()>
                        <select class="input prop-field-select" aria-label=label2 title=ui.t("prop-field-help")
                            on:change=move |e| {
                                let v = event_target_value(&e);
                                let item = item.get_value();
                                match v.as_str() {
                                    "@keep" => {}
                                    "@url" => set(name.get_value(), Some(format!("{item}.url"))),
                                    // Valeur écrite : celle lue à l'instant, modifiable ensuite.
                                    "@write" => {
                                        let v = match (kind, read()) {
                                            (PropKind::Switch, v) => Some(v.is_some_and(|v| v != Value::Bool(false)).to_string()),
                                            (PropKind::List, _) => None,
                                            (_, Some(Value::String(t))) if !t.is_empty() => Some(quote(&t)),
                                            _ => None,
                                        };
                                        set(name.get_value(), v);
                                    }
                                    other => if let Some(f) = other.strip_prefix("f:") {
                                        set(name.get_value(), Some(bound_expr(&item, f, None, None)));
                                    },
                                }
                            }>
                            {shown.map(|t| view! { <option value="@keep" selected=true>{t}</option> })}
                            {move || {
                                let mut opts = options();
                                // Champ relié absent du schéma : gardé dans le menu.
                                if let Some(f) = field.get_value()
                                    && !opts.iter().any(|(n, _)| *n == f)
                                {
                                    opts.insert(0, (f.clone(), revel_edit::overrides::field_label(&f)));
                                }
                                opts.into_iter().map(|(n, l)| {
                                    let v = format!("f:{n}");
                                    let on = current.with_value(|c| *c == v);
                                    view! { <option value=v prop:selected=on>{l}</option> }
                                }).collect_view()
                            }}
                            {(kind != PropKind::List && kind != PropKind::Switch).then(|| view! {
                                <option value="@url" prop:selected=current.with_value(|c| c == "@url")>{ui.t("prop-field-page")}</option>
                            })}
                            <option value="@write">{ui.t("prop-write")}</option>
                        </select>
                        {move || read().map(|v| {
                            let t = preview_text(&v);
                            let tip = t.clone();
                            view! { <span class="prop-preview" title=tip>{t}</span> }
                        })}
                    </span>
                </div>
                {move || date.get().then(|| view! {
                    <label class="field" data-bound="block-bound-date">
                        <span class="label">{ui.t("block-bound-date")}</span>
                        <div class="select">
                            <select class="input" on:change=move |e| {
                                let v = event_target_value(&e);
                                let fmt = (!v.is_empty()).then_some(v);
                                if let Some(f) = field.get_value() {
                                    set(name.get_value(), Some(bound_expr(&item.get_value(), &f, fallback.get_value().as_deref(), fmt.as_deref())));
                                }
                            }>
                                <option value="" prop:selected=cur_fmt.with_value(String::is_empty)>{ui.t("block-bound-date-raw")}</option>
                                {revel_edit::dates::FORMATS.iter().map(|(f, _)| {
                                    let on = cur_fmt.with_value(|c| revel_edit::dates::format_of(c).as_deref() == Some(*f));
                                    view! { <option value=*f prop:selected=on>{revel_edit::dates::example(f, &lang)}</option> }
                                }).collect_view()}
                            </select>
                        </div>
                    </label>
                })}
            </div>
        }
        .into_any();
    }
    let can_bind = list.is_some();
    // Source choisie, propriété ni reliée ni écrite : à relier.
    let unbound = can_bind && arg.is_none() && p.kind != PropKind::Color;
    let field_menu = move |e: leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open};
        let Some(((collection, item), cols, _)) = list.clone() else {
            return;
        };
        // Adresse de la page du contenu, puis ses champs.
        let mut items: Vec<CtxItem> = Vec::new();
        if p.kind != PropKind::List && p.kind != PropKind::Switch {
            let expr = format!("{item}.url");
            items.push(
                CtxItem::new(ui.now("component-field-url", &[]), move || {
                    set(name.get_value(), Some(expr.clone()))
                })
                .icon(icons::link),
            );
        }
        let fields: Vec<CtxItem> = cols.with_untracked(|cs| {
            cs.iter()
                .find(|c| c.id == collection)
                .map(|c| {
                    c.fields
                        .iter()
                        // Propriété liste : champs à plusieurs valeurs ; sinon une valeur.
                        .filter(|f| field_fits(p.kind, f))
                        .map(|f| {
                            let expr = format!("{item}.data.{}", f.name);
                            CtxItem::new(
                                f.label.clone().unwrap_or_else(|| f.name.clone()),
                                move || set(name.get_value(), Some(expr.clone())),
                            )
                            .icon(icons::database)
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
        items.extend(fields);
        open(ui, &e, items);
    };

    let reset = move || {
        overridden.then(|| {
            view! {
                <button class="icon-btn icon-btn--small" title=ui.t("component-prop-reset") aria-label=ui.t("component-prop-reset")
                    on:click=move |_| set(name.get_value(), None)>{icons::reset()}</button>
            }
        })
    };
    if p.kind == PropKind::Switch {
        let on = match arg.as_deref().map(str::trim) {
            Some("true") => true,
            Some("false") => false,
            _ => default_bool(&p),
        };
        let dflt = default_bool(&p);
        return view! {
            <label class="view-line prop-row" data-prop=p.name.clone()>
                <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                {can_bind.then(|| view! {
                    <button type="button" class="icon-btn icon-btn--small bind-field" class:is-missing=unbound title=ui.t("component-bind-field") aria-label=ui.t("component-bind-field")
                        on:click=field_menu.clone()>{icons::database()}</button>
                })}
                {reset()}
                <span class="switch">
                    <input type="checkbox" prop:checked=on
                        on:change=move |e| {
                            let v = event_target_checked(&e);
                            set(name.get_value(), (v != dflt).then(|| v.to_string()));
                        }/>
                    <span class="switch-track"></span>
                </span>
            </label>
        }
        .into_any();
    }
    // Nombre (D-196) : écrit tel quel, sans guillemets.
    if p.kind == PropKind::Number && arg.as_deref().is_none_or(|a| num_value(a).is_some()) {
        let shown = arg.clone().unwrap_or_default();
        return view! {
            <div class="prop-row" data-prop=p.name.clone()>
                <Field label=label.clone() value=shown placeholder=default_text(&p)
                    on_commit=move |v: String| set(name.get_value(), num_value(&v).map(|n| n.to_string()))>
                    {can_bind.then(|| view! {
                        <button type="button" class="icon-btn icon-btn--small bind-field" class:is-missing=unbound title=ui.t("component-bind-field") aria-label=ui.t("component-bind-field")
                            on:click=field_menu.clone()>{icons::database()}</button>
                    })}
                    {reset()}
                </Field>
            </div>
        }
        .into_any();
    }
    // Choix (D-196) : une des valeurs proposées par le composant.
    if p.kind == PropKind::Choice && arg.as_deref().is_none_or(|a| literal(a).is_some()) {
        let dflt = default_text(&p);
        let current = arg
            .as_deref()
            .and_then(literal)
            .unwrap_or_else(|| dflt.clone());
        let mut options = p.options.clone();
        if !current.is_empty() && !options.contains(&current) {
            options.push(current.clone());
        }
        return view! {
            <div class="prop-row" data-prop=p.name.clone()>
                <div class="field">
                    <span class="label prop-color-label">
                        <span class="prop-label" class:is-set=overridden>{label.clone()}</span>
                        {reset()}
                    </span>
                    <div class="select">
                        <select class="input prop-choice" aria-label=label
                            on:change=move |e| {
                                let v = event_target_value(&e);
                                set(name.get_value(), (v != dflt).then(|| quote(&v)));
                            }>
                            {options.into_iter().map(|o| {
                                let on = o == current;
                                view! { <option value=o.clone() selected=on>{o.clone()}</option> }
                            }).collect_view()}
                        </select>
                    </div>
                </div>
            </div>
        }
        .into_any();
    }
    // Icône (D-196) : une icône de Revel.
    if p.kind == PropKind::Icon && arg.as_deref().is_none_or(|a| literal(a).is_some()) {
        let dflt = default_text(&p);
        let current = arg
            .as_deref()
            .and_then(literal)
            .unwrap_or_else(|| dflt.clone());
        return view! {
            <div class="prop-row" data-prop=p.name.clone()>
                <div class="field">
                    <span class="label prop-color-label">
                        <span class="prop-label" class:is-set=overridden>{label.clone()}</span>
                        {reset()}
                    </span>
                    {icon_choice(ui, current, label, move |v: String| set(name.get_value(), (v != dflt).then(|| quote(&v))))}
                </div>
            </div>
        }
        .into_any();
    }
    // Couleur du site (D-184) : choisie dans la liste des couleurs du site, jamais écrite.
    if p.kind == PropKind::Color && arg.as_deref().is_none_or(|a| literal(a).is_some()) {
        let current = arg.as_deref().and_then(literal).unwrap_or_default();
        let dflt = site_color_names(ed)
            .into_iter()
            .find(|(n, _, _)| *n == default_text(&p))
            .map(|(_, l, _)| l)
            .unwrap_or_else(|| default_text(&p));
        return view! {
            <div class="prop-row" data-prop=p.name.clone()>
                <div class="field">
                    <span class="label prop-color-label">
                        <span class="prop-label" class:is-set=overridden>{label.clone()}</span>
                        {reset()}
                    </span>
                    {color_select(ed, current, dflt, label, move |v| set(name.get_value(), v.map(|v| quote(&v))))}
                </div>
            </div>
        }
        .into_any();
    }
    // Propriété liste écrite à la main (D-139) : une ligne par valeur.
    if p.kind == PropKind::List
        && let Some(items) = arg.as_deref().map_or(Some(Vec::new()), list_literal)
    {
        let items = Signal::stored(items);
        return view! {
            <div class="prop-row prop-row--list" data-prop=p.name.clone()>
                <div class="prop-list-head">
                    <span class="prop-label">{kind_icon(p.kind)}{label}</span>
                    {can_bind.then(|| view! {
                        <button type="button" class="icon-btn icon-btn--small bind-field" class:is-missing=unbound title=ui.t("component-bind-field") aria-label=ui.t("component-bind-field")
                            on:click=field_menu.clone()>{icons::database()}</button>
                    })}
                    {reset()}
                </div>
                <crate::cmsfields::RowsEditor items=items set=Callback::new(move |v: Vec<String>| {
                    set(name.get_value(), (!v.is_empty()).then(|| format!("[{}]", v.iter().map(|x| quote(x)).collect::<Vec<_>>().join(", "))))
                })/>
            </div>
        }
        .into_any();
    }
    // Valeur calculée (expression) : modifiée telle quelle.
    let expr = p.kind == PropKind::List || arg.as_deref().is_some_and(|a| literal(a).is_none());
    let shown = match &arg {
        Some(a) => literal(a).unwrap_or_else(|| a.clone()),
        None => String::new(),
    };
    let field_label = if expr {
        ui.now("component-prop-computed", &[("label", label.clone())])
    } else {
        label.clone()
    };
    let placeholder = default_text(&p);
    let image = p.kind == PropKind::Image;
    view! {
        <div class="prop-row" data-prop=p.name.clone()>
            <Field label=field_label value=shown mono=expr placeholder=placeholder
                on_commit=move |v: String| {
                    let v2 = v.trim().to_string();
                    set(name.get_value(), if v2.is_empty() { None } else if expr { Some(v2) } else { Some(quote(&v)) })
                }>
                {image.then(|| view! {
                    <button type="button" class="icon-btn icon-btn--small" title=ui.t("picker-choose-image") aria-label=ui.t("picker-choose-image")
                        on:click=move |_| crate::library::pick_image(ui, crate::library::media_shelf(ed), move |url| set(name.get_value(), Some(quote(&url))))>
                        {icons::image()}</button>
                })}
                {can_bind.then(|| view! {
                    <button type="button" class="icon-btn icon-btn--small bind-field" class:is-missing=unbound title=ui.t("component-bind-field") aria-label=ui.t("component-bind-field")
                        on:click=field_menu.clone()>{icons::database()}</button>
                })}
                {reset()}
            </Field>
        </div>
    }
    .into_any()
}

/// À l'ouverture d'un site (D-191) : les composants à propriété « Contenu » sont convertis, en une
/// étape annulable — une propriété par champ lu, chaque utilisation reliée à son contenu. Les
/// valeurs par défaut viennent du premier contenu de chaque collection.
pub fn convert_entry_props(ui: Ui, ed: Ed) {
    let needed = ed.doc.with_untracked(|d| {
        d.as_ref().is_some_and(|d| {
            d.components
                .iter()
                .any(|c| c.props.iter().any(|p| p.kind == PropKind::Entry))
        })
    });
    if !needed {
        return;
    }
    let path = ed.path.get_value();
    leptos::task::spawn_local(async move {
        let Ok(cols) = crate::api::cms_collections(&path).await else {
            return;
        };
        let schema = schema_of(&cols);
        let cmd = ed.doc.with_untracked(|d| {
            revel_edit::entryprops::convert(d.as_ref()?, &schema, &ui.now("prop-page", &[]))
        });
        if let Some(cmd) = cmd {
            let n = match &cmd {
                Command::Batch { commands } => commands
                    .iter()
                    .filter(|c| matches!(c, Command::SetComponentProps { .. }))
                    .count(),
                _ => 0,
            };
            ed.apply(ui, cmd, false);
            if n > 0 {
                ui.fail(
                    revel_protocol::Failure::new("info-components-converted")
                        .arg("n", n.to_string()),
                );
            }
        }
    });
}

/// Champs des collections (nom, nature, valeur du premier contenu), pour nommer et typer les
/// propriétés tirées du contenu (D-191, D-193).
fn schema_of(
    cols: &[revel_protocol::CmsCollection],
) -> BTreeMap<String, revel_edit::entryprops::CollectionInfo> {
    cols.iter()
        .map(|c| {
            let first = c.entries.first();
            let fields = c
                .fields
                .iter()
                .map(|f| {
                    (
                        f.name.clone(),
                        revel_edit::entryprops::FieldInfo {
                            label: f.label.clone(),
                            kind: f.kind.clone(),
                            list: f.list,
                            sample: first.and_then(|e| e.data.get(&f.name).cloned()),
                            // Première valeur remplie parmi les contenus : nature du champ.
                            example: c.entries.iter().find_map(|e| {
                                e.data
                                    .get(&f.name)
                                    .filter(|v| !v.is_null() && v.as_str() != Some(""))
                                    .cloned()
                            }),
                        },
                    )
                })
                .collect();
            (
                c.id.clone(),
                revel_edit::entryprops::CollectionInfo {
                    fields,
                    url: first.and_then(|e| e.url.clone()),
                },
            )
        })
        .collect()
}

/// Propriété du composant ouvert (ou du gabarit) qui remplit l'élément `at` : son texte, son
/// image ou son lien. Libellé de la propriété, pour les calques.
pub(crate) fn prop_filling(d: &revel_model::Site, at: &Target) -> Option<String> {
    let props = Holder::of(at)?.visible(d)?;
    let named = |e: &str| {
        let e = e.trim();
        props
            .iter()
            .find(|p| p.name == e)
            .map(|p| p.label().to_string())
    };
    let tpl = |v: &str| {
        v.trim()
            .strip_prefix("{{")
            .and_then(|x| x.strip_suffix("}}"))
            .and_then(named)
    };
    match revel_edit::address::node(d, at).ok()? {
        Node::Element {
            attrs, children, ..
        } => match children.as_slice() {
            [Node::Out { expr }] => named(expr),
            _ => None,
        }
        .or_else(|| {
            attrs
                .iter()
                .filter(|a| matches!(a.name.as_str(), "src" | "href"))
                .find_map(|a| tpl(a.value.as_deref()?))
        }),
        Node::Image(img) => named(&img.src),
        _ => None,
    }
}

// ------------------------------------------------------------------ composant ouvert

/// Propriétés du composant ouvert : nom affiché, valeur par défaut, suppression ; ajout par type.
pub fn props_section(ui: Ui, ed: Ed, holder: Holder) -> AnyView {
    let layout = matches!(holder, Holder::Layout(_));
    let holder = StoredValue::new(holder);
    let props = Memo::new(move |_| {
        ed.doc
            .with(|d| d.as_ref().and_then(|d| holder.with_value(|h| h.own(d))))
    });
    let write = move |props: Vec<PropDef>| ed.apply(ui, holder.with_value(|h| h.set(props)), false);
    let add = move |e: leptos::ev::MouseEvent| {
        use crate::ctxmenu::{CtxItem, open};
        let items = KINDS
            .into_iter()
            // Gabarit : valeurs réglées dans les réglages d'une page (pas de liste).
            .filter(|k| !(layout && matches!(*k, PropKind::List | PropKind::Entry)))
            .map(|k| {
                CtxItem::new(ui.now(kind_key(k), &[]), move || {
                    let Some(mut list) = props.get_untracked() else {
                        return;
                    };
                    let visible = ed
                        .doc
                        .with_untracked(|d| {
                            d.as_ref().and_then(|d| holder.with_value(|h| h.visible(d)))
                        })
                        .unwrap_or_else(|| list.clone());
                    let mut prop = new_prop(ui, &visible, k, None);
                    if k == PropKind::Color {
                        prop.default = site_color_names(ed)
                            .into_iter()
                            .next()
                            .map(|(n, _, _)| Value::String(n));
                        let carry = holder.with_value(|h| match h {
                            Holder::Component(id) => {
                                component(ed, id).and_then(|c| carry_color(&c, &prop.name))
                            }
                            Holder::Layout(_) => None,
                        });
                        list.push(prop);
                        let set = holder.with_value(|h| h.set(list));
                        let commands = std::iter::once(set).chain(carry).collect();
                        ed.apply(ui, Command::Batch { commands }, false);
                        return;
                    }
                    list.push(prop);
                    write(list);
                })
                .icon(kind_icon_fn(k))
            })
            .collect();
        open(ui, &e, items);
    };
    let (title, empty) = if layout {
        ("layout-props", "layout-props-empty")
    } else {
        ("component-props", "component-props-empty")
    };
    view! {
        <div class="insp-group component-props">
            <div class="component-props-head">
                <p class="panel-title" title=ui.t(empty)>{ui.t(title)}</p>
                <button class="icon-btn icon-btn--small" title=ui.t("component-prop-add") aria-label=ui.t("component-prop-add")
                    on:click=add>{icons::plus()}</button>
            </div>
            {move || props.get().map(|list| {
                if list.is_empty() {
                    return ().into_any();
                }
                list.iter().enumerate().map(|(i, p)| {
                    let kind = p.kind;
                    let pname = StoredValue::new(p.name.clone());
                    let set_prop = move |f: Box<dyn Fn(&mut PropDef)>| {
                        let Some(mut list) = props.get_untracked() else { return };
                        if let Some(p) = list.get_mut(i) {
                            f(p);
                        }
                        write(list);
                    };
                    let remove = move |_| {
                        let cmd = ed.doc.with_untracked(|d| {
                            holder.with_value(|h| h.remove(d.as_ref()?, &pname.get_value()))
                        });
                        if let Some(cmd) = cmd {
                            ed.apply(ui, cmd, false);
                        }
                    };
                    let default_view = if kind == PropKind::Entry {
                        let cols = crate::blocks::collections(ed);
                        let col = p.collection.clone().unwrap_or_default();
                        let col2 = col.clone();
                        let slug = p.default.as_ref().and_then(|d| d.as_str()).unwrap_or_default().to_string();
                        view! {
                            <div class="style-row" data-row="prop-collection">
                                <span class="style-label" title=ui.t("prop-entry-collection-help")><span class="style-label-text">{ui.t("prop-entry-collection")}</span></span>
                                <div class="style-control"><div class="select">
                                    <select class="input prop-entry-collection" aria-label=ui.t("prop-entry-collection")
                                        on:change=move |e| {
                                            let v = event_target_value(&e);
                                            set_prop(Box::new(move |p| {
                                                p.collection = (!v.is_empty()).then(|| v.clone());
                                                p.default = None;
                                            }));
                                        }>
                                        <option value="" selected=col.is_empty()>{ui.t("prop-entry-choose")}</option>
                                        {move || cols.with(|cs| cs.iter().filter(|c| !c.id.contains('+')).map(|c| {
                                            let on = c.id == col;
                                            view! { <option value=c.id.clone() selected=on>{c.label.clone().unwrap_or_else(|| crate::site::capitalize(&c.id))}</option> }
                                        }).collect_view())}
                                    </select>
                                </div></div>
                            </div>
                            {(!col2.is_empty()).then(|| view! {
                                <div class="style-row" data-row="prop-entry">
                                    <span class="style-label" title=ui.t("prop-entry-default-help")><span class="style-label-text">{ui.t("prop-entry-content")}</span></span>
                                    <div class="style-control"><div class="select">
                                        <select class="input prop-entry-default" aria-label=ui.t("prop-entry-content")
                                            on:change=move |e| {
                                                let v = event_target_value(&e);
                                                set_prop(Box::new(move |p| p.default = (!v.is_empty()).then(|| Value::String(v.clone()))));
                                            }>
                                            <option value="" selected=slug.is_empty()>{ui.t("prop-entry-first")}</option>
                                            {move || entries_of(cols, &col2).into_iter().map(|(s, t)| {
                                                let on = s == slug;
                                                view! { <option value=s selected=on>{t}</option> }
                                            }).collect_view()}
                                        </select>
                                    </div></div>
                                </div>
                            })}
                        }.into_any()
                    } else if kind == PropKind::Switch {
                        let on = default_bool(p);
                        view! {
                            <label class="view-line">
                                <span class="muted">{ui.t("component-prop-default")}</span>
                                <span class="switch">
                                    <input type="checkbox" prop:checked=on
                                        on:change=move |e| {
                                            let v = event_target_checked(&e);
                                            set_prop(Box::new(move |p| p.default = Some(Value::Bool(v))));
                                        }/>
                                    <span class="switch-track"></span>
                                </span>
                            </label>
                        }.into_any()
                    } else if kind == PropKind::Number {
                        view! {
                            <Field label=ui.t("component-prop-default") value=default_text(p)
                                on_commit=move |v: String| set_prop(Box::new(move |p| p.default = num_value(&v)))/>
                        }.into_any()
                    } else if kind == PropKind::Choice {
                        let options = Signal::stored(p.options.clone());
                        let list = p.options.clone();
                        let dflt = default_text(p);
                        view! {
                            <div class="prop-options" title=ui.t("prop-choice-options-help")>
                                <span class="label">{ui.t("prop-choice-options")}</span>
                                <crate::cmsfields::RowsEditor items=options set=Callback::new(move |v: Vec<String>| {
                                    set_prop(Box::new(move |p| {
                                        let v: Vec<String> = v.iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                                        // Valeur par défaut retirée de la liste : la première.
                                        let keep = p.default.as_ref().and_then(|d| d.as_str()).is_some_and(|d| v.iter().any(|x| x == d));
                                        if !keep {
                                            p.default = v.first().cloned().map(Value::String);
                                        }
                                        p.options = v.clone();
                                    }))
                                })/>
                            </div>
                            {(!list.is_empty()).then(|| view! {
                                <label class="field">
                                    <span class="label">{ui.t("component-prop-default")}</span>
                                    <div class="select">
                                        <select class="input prop-choice-default" aria-label=ui.t("component-prop-default")
                                            on:change=move |e| {
                                                let v = event_target_value(&e);
                                                set_prop(Box::new(move |p| p.default = Some(Value::String(v.clone()))));
                                            }>
                                            {list.iter().map(|o| {
                                                let on = *o == dflt;
                                                view! { <option value=o.clone() selected=on>{o.clone()}</option> }
                                            }).collect_view()}
                                        </select>
                                    </div>
                                </label>
                            })}
                        }.into_any()
                    } else if kind == PropKind::Icon {
                        view! {
                            <div class="field">
                                <span class="label">{ui.t("component-prop-default")}</span>
                                {icon_choice(ui, default_text(p), ui.now("component-prop-default", &[]),
                                    move |v: String| set_prop(Box::new(move |p| p.default = Some(Value::String(v.clone())))))}
                            </div>
                        }.into_any()
                    } else if kind == PropKind::Color {
                        view! {
                            <label class="field">
                                <span class="label">{ui.t("component-prop-default")}</span>
                                {color_select(ed, default_text(p), ui.now("prop-color-none", &[]), ui.now("component-prop-default", &[]),
                                    move |v| set_prop(Box::new(move |p| p.default = v.clone().map(Value::String))))}
                            </label>
                        }.into_any()
                    } else {
                        view! {
                            <Field label=ui.t("component-prop-default") value=default_text(p)
                                on_commit=move |v: String| set_prop(Box::new(move |p| {
                                    p.default = (!v.is_empty()).then(|| Value::String(v.clone()));
                                }))/>
                        }.into_any()
                    };
                    view! {
                        <div class="prop-def" data-prop=p.name.clone()>
                            <div class="prop-def-head">
                                <span class="prop-kind" title=ui.t(kind_key(kind))>{kind_icon(kind)}</span>
                                <input class="input prop-def-label" prop:value=p.label().to_string()
                                    aria-label=ui.t("component-prop-label") title=format!("{{{{ {} }}}}", p.name)
                                    on:change=move |e| {
                                        let v = event_target_value(&e).trim().to_string();
                                        set_prop(Box::new(move |p| p.label = (!v.is_empty()).then(|| v.clone())));
                                    }/>
                                <button class="icon-btn icon-btn--small" title=ui.t("component-prop-remove") aria-label=ui.t("component-prop-remove")
                                    on:click=remove>{icons::trash()}</button>
                            </div>
                            {default_view}
                        </div>
                    }.into_any()
                }).collect_view().into_any()
            })}
        </div>
    }
    .into_any()
}

/// Nouvelle propriété de type `k` (nom libre, libellé numéroté, valeur par défaut donnée).
pub(crate) fn new_prop(ui: Ui, props: &[PropDef], k: PropKind, default: Option<Value>) -> PropDef {
    new_prop_named(ui, props, k, default, None)
}

/// Identifiant de variable tiré d'un nom de calque (`Titre de carte` -> `titre_de_carte`) ; rien
/// d'utilisable : `None`.
fn ident_of(label: &str) -> Option<String> {
    let slug = revel_model::slugify(label).replace('-', "_");
    let slug = slug.trim_matches('_').to_string();
    let slug = if slug.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("v_{slug}")
    } else {
        slug
    };
    revel_model::valid_prop_name(&slug).then_some(slug)
}

/// Nouvelle variable nommée d'après le calque relié (`layer`, D-196, comme Framer) ; sans calque :
/// d'après son type.
pub(crate) fn new_prop_named(
    ui: Ui,
    props: &[PropDef],
    k: PropKind,
    default: Option<Value>,
    layer: Option<String>,
) -> PropDef {
    let layer = layer
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty());
    let base = layer
        .as_deref()
        .and_then(ident_of)
        .unwrap_or_else(|| base_name(k).to_string());
    let name = revel_edit::make::free_name(props, &base);
    let base_label = layer.unwrap_or_else(|| ui.now(kind_key(k), &[]));
    let mut label = base_label.clone();
    let mut n = 2;
    while props.iter().any(|p| p.label() == label) {
        label = format!("{base_label} {n}");
        n += 1;
    }
    PropDef {
        options: Vec::new(),
        collection: None,
        name,
        label: Some(label),
        kind: k,
        default: default.or_else(|| match k {
            PropKind::Switch => Some(Value::Bool(true)),
            // Icône : la première de Revel, pour que l'élément relié dessine quelque chose.
            PropKind::Icon => revel_edit::icons::ICONS
                .first()
                .map(|(n, _)| Value::String(n.to_string())),
            _ => None,
        }),
    }
}

/// Ce qu'un élément du composant peut relier à une propriété.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Bind {
    Text,
    Src,
    Href,
    Visible,
    /// Dessin d'un élément icône (variable « Icône », D-196).
    Icon,
}

impl Bind {
    /// Type de la variable créée par « Nouvelle variable ».
    fn kind(self) -> PropKind {
        match self {
            Bind::Text => PropKind::Text,
            Bind::Src => PropKind::Image,
            Bind::Href => PropKind::Link,
            Bind::Visible => PropKind::Switch,
            Bind::Icon => PropKind::Icon,
        }
    }
    /// Types de variable qui peuvent s'y relier : un texte reçoit aussi un nombre ou un choix.
    fn accepts(self, k: PropKind) -> bool {
        k == self.kind() || (self == Bind::Text && matches!(k, PropKind::Number | PropKind::Choice))
    }
    fn key(self) -> &'static str {
        match self {
            Bind::Text => "component-bind-text",
            Bind::Src => "component-bind-image",
            Bind::Href => "component-bind-link",
            Bind::Visible => "component-bind-visibility",
            Bind::Icon => "component-bind-icon",
        }
    }
}

/// Expression du dessin d'une icône lue dans la variable `name`.
fn icon_expr(name: &str) -> String {
    format!("{name} | icon")
}

/// `{{ name }}` seul.
fn only_var(text: &str) -> Option<String> {
    let t = text.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
    revel_model::valid_prop_name(t).then(|| t.to_string())
}

/// État d'une liaison pour le nœud `at` : `Some(Some(prop))` reliée, `Some(None)` possible
/// (valeur actuelle écrite), `None` sans objet.
fn bind_state(
    site: &revel_model::Site,
    props: &[PropDef],
    at: &Target,
    b: Bind,
) -> Option<(Option<String>, String)> {
    let node = revel_edit::address::node(site, at).ok()?;
    let is_prop = |name: &str| props.iter().any(|p| p.name == name && b.accepts(p.kind));
    match (b, node) {
        (Bind::Text, Node::Element { children, .. }) => match children.as_slice() {
            [Node::Text { text }] => Some((None, text.clone())),
            [Node::Out { expr }] if is_prop(expr.trim()) => {
                Some((Some(expr.trim().to_string()), String::new()))
            }
            _ => None,
        },
        (Bind::Src, Node::Image(img)) => {
            if is_prop(img.src.trim()) {
                Some((Some(img.src.trim().to_string()), String::new()))
            } else {
                literal(&img.src).map(|t| (None, t))
            }
        }
        (Bind::Href, Node::Element { tag, attrs, .. }) if tag.eq_ignore_ascii_case("a") => {
            let v = attrs
                .iter()
                .find(|a| a.name == "href")
                .and_then(|a| a.value.clone())
                .unwrap_or_default();
            match only_var(&v) {
                Some(n) if is_prop(&n) => Some((Some(n), String::new())),
                _ if !v.contains("{{") && !v.contains("{%") => Some((None, v)),
                _ => None,
            }
        }
        (Bind::Icon, Node::Element { tag, attrs, .. }) if tag.eq_ignore_ascii_case("svg") => {
            let v = attrs
                .iter()
                .find(|a| a.name == "data-icon")
                .and_then(|a| a.value.clone())?;
            match only_var(&v) {
                Some(n) if is_prop(&n) => Some((Some(n), String::new())),
                // Icône importée (fichier) : dessin propre, pas un nom d'icône de Revel.
                _ if revel_edit::icons::icon(&v).is_some() => Some((None, v)),
                _ => None,
            }
        }
        (Bind::Visible, _) => {
            let (parent, _) = revel_edit::address::locate(site, at).ok()?;
            let wrapper = parent.node.as_ref().and_then(|id| {
                revel_edit::address::node(
                    site,
                    &Target {
                        owner: at.owner.clone(),
                        node: id.clone(),
                    },
                )
                .ok()
            });
            match wrapper {
                Some(Node::If {
                    cond,
                    then,
                    otherwise,
                    ..
                }) if parent.branch == Branch::Then
                    && then.len() == 1
                    && otherwise.is_empty()
                    && is_prop(cond.trim()) =>
                {
                    Some((Some(cond.trim().to_string()), String::new()))
                }
                _ => Some((None, String::new())),
            }
        }
        _ => None,
    }
}

/// Commandes qui relient le nœud `at` à la propriété `name` (`new` : propriétés du composant
/// avec la nouvelle).
fn bind_commands(
    site: &revel_model::Site,
    holder: &Holder,
    at: &Target,
    b: Bind,
    name: &str,
    new: Option<Vec<PropDef>>,
) -> Option<Command> {
    let mut commands = Vec::new();
    if let Some(props) = new {
        commands.push(holder.set(props));
    }
    match (b, revel_edit::address::node(site, at).ok()?) {
        (Bind::Text, _) => commands.push(Command::Replace {
            parent: Parent::of(at, Branch::Children),
            index: 0,
            node: Node::Out {
                expr: name.to_string(),
            },
        }),
        (Bind::Src, Node::Image(img)) => {
            let mut image = img.clone();
            image.src = name.to_string();
            commands.push(Command::SetImage {
                at: at.clone(),
                image,
            });
        }
        (Bind::Href, _) => commands.push(Command::SetAttr {
            at: at.clone(),
            name: "href".into(),
            value: Some(format!("{{{{ {name} }}}}")),
        }),
        (Bind::Icon, Node::Element { children, .. }) => {
            commands.push(Command::SetAttr {
                at: at.clone(),
                name: "data-icon".into(),
                value: Some(format!("{{{{ {name} }}}}")),
            });
            let out = Node::Out {
                expr: icon_expr(name),
            };
            commands.push(if children.is_empty() {
                Command::Insert {
                    parent: Parent::of(at, Branch::Children),
                    index: 0,
                    node: out,
                }
            } else {
                Command::Replace {
                    parent: Parent::of(at, Branch::Children),
                    index: 0,
                    node: out,
                }
            });
        }
        (Bind::Visible, _) => {
            let (parent, index) = revel_edit::address::locate(site, at).ok()?;
            let if_id = revel_model::ids::fresh(&site.node_ids());
            commands.push(Command::Insert {
                parent,
                index,
                node: Node::If {
                    id: Some(if_id.clone()),
                    cond: name.to_string(),
                    then: Vec::new(),
                    otherwise: Vec::new(),
                },
            });
            commands.push(Command::Move {
                at: at.clone(),
                to: Parent {
                    owner: at.owner.clone(),
                    node: Some(if_id),
                    branch: Branch::Then,
                },
                index: 0,
            });
        }
        _ => return None,
    }
    Some(Command::Batch { commands })
}

/// Commandes qui délient le nœud `at` de sa propriété (sa valeur par défaut est écrite à la
/// place).
fn unbind_commands(
    site: &revel_model::Site,
    props: &[PropDef],
    at: &Target,
    b: Bind,
    name: &str,
) -> Option<Command> {
    let dflt = props
        .iter()
        .find(|p| p.name == name)
        .map(default_text)
        .unwrap_or_default();
    Some(match (b, revel_edit::address::node(site, at).ok()?) {
        (Bind::Text, _) => Command::Replace {
            parent: Parent::of(at, Branch::Children),
            index: 0,
            node: Node::Text { text: dflt },
        },
        (Bind::Src, Node::Image(img)) => {
            let mut image = img.clone();
            image.src = quote(&dflt);
            Command::SetImage {
                at: at.clone(),
                image,
            }
        }
        (Bind::Href, _) => Command::SetAttr {
            at: at.clone(),
            name: "href".into(),
            value: Some(dflt),
        },
        (Bind::Icon, _) => {
            let name = if revel_edit::icons::icon(&dflt).is_some() {
                dflt
            } else {
                revel_edit::icons::ICONS[0].0.to_string()
            };
            Command::Batch {
                commands: vec![
                    Command::SetAttr {
                        at: at.clone(),
                        name: "data-icon".into(),
                        value: Some(name.clone()),
                    },
                    Command::Replace {
                        parent: Parent::of(at, Branch::Children),
                        index: 0,
                        node: Node::Html {
                            html: revel_edit::icons::icon(&name)
                                .unwrap_or_default()
                                .to_string(),
                        },
                    },
                ],
            }
        }
        (Bind::Visible, _) => {
            let (parent, _) = revel_edit::address::locate(site, at).ok()?;
            let if_at = Target {
                owner: at.owner.clone(),
                node: parent.node?,
            };
            let (if_parent, if_index) = revel_edit::address::locate(site, &if_at).ok()?;
            Command::Batch {
                commands: vec![
                    Command::Move {
                        at: at.clone(),
                        to: if_parent,
                        index: if_index,
                    },
                    Command::Remove { at: if_at },
                ],
            }
        }
        _ => return None,
    })
}

/// Menu de liaison de l'élément `at` (D-196) : nouvelle variable nommée d'après le calque,
/// variables existantes du bon type ; déjà relié (`linked`) : « Délier » en tête.
fn bind_menu(
    ui: Ui,
    ed: Ed,
    holder: StoredValue<Holder>,
    at: StoredValue<Target>,
    b: Bind,
    e: &leptos::ev::MouseEvent,
    (linked, current): (Option<String>, String),
) {
    use crate::ctxmenu::{CtxItem, open};
    let Some((own, visible, layer)) = ed.doc.with_untracked(|d| {
        let d = d.as_ref()?;
        let layer = revel_edit::address::node(d, &at.get_value())
            .ok()
            .map(|n| crate::editor::node_label(ui, n));
        holder.with_value(|h| Some((h.own(d)?, h.visible(d)?, layer)))
    }) else {
        return;
    };
    let run = move |name: String, new: Option<Vec<PropDef>>| {
        let cmd = ed.doc.with_untracked(|d| {
            holder.with_value(|h| bind_commands(d.as_ref()?, h, &at.get_value(), b, &name, new))
        });
        if let Some(cmd) = cmd {
            ed.apply(ui, cmd, false);
        }
    };
    let mut items = Vec::new();
    if let Some(name) = linked.clone() {
        items.push(
            CtxItem::new(ui.now("component-unbind", &[]), move || {
                let cmd = ed.doc.with_untracked(|d| {
                    let d = d.as_ref()?;
                    let props = holder.with_value(|h| h.visible(d))?;
                    unbind_commands(d, &props, &at.get_value(), b, &name)
                });
                if let Some(cmd) = cmd {
                    ed.apply(ui, cmd, false);
                }
            })
            .icon(icons::close),
        );
    } else {
        // Nom de la variable : celui du calque (comme Framer), sinon son type.
        let label = layer
            .clone()
            .filter(|l| !l.trim().is_empty())
            .unwrap_or_else(|| ui.now(kind_key(b.kind()), &[]));
        let (own, visible) = (own.clone(), visible.clone());
        items.push(
            CtxItem::new(
                ui.now("component-bind-new", &[("label", label)]),
                move || {
                    let default = match b {
                        Bind::Visible => Some(Value::Bool(true)),
                        _ => (!current.is_empty()).then(|| Value::String(current.clone())),
                    };
                    let p = new_prop_named(ui, &visible, b.kind(), default, layer.clone());
                    let mut props = own.clone();
                    let name = p.name.clone();
                    props.push(p);
                    run(name, Some(props));
                },
            )
            .icon(icons::plus),
        );
    }
    for p in visible
        .iter()
        .filter(|p| b.accepts(p.kind) && Some(&p.name) != linked.as_ref())
    {
        let name = p.name.clone();
        items.push(
            CtxItem::new(p.label().to_string(), move || run(name.clone(), None))
                .icon(kind_icon_fn(p.kind)),
        );
    }
    open(ui, e, items);
}

/// Bouton de liaison posé à côté du champ d'un élément du composant ouvert (D-196, comme le « + »
/// de Framer) : relie ce champ à une variable ; relié, il porte le nom de la variable et son menu
/// délie. Hors d'un composant, ou champ sans objet : rien.
pub(crate) fn bind_button(ui: Ui, ed: Ed, at: &Target, b: Bind) -> Option<AnyView> {
    let holder = StoredValue::new(Holder::of(at)?);
    let at = StoredValue::new(at.clone());
    let state = move || {
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            let props = holder.with_value(|h| h.visible(d))?;
            // Gabarit : seulement s'il a des variables (créées dans les réglages d'une page).
            if holder.with_value(|h| matches!(h, Holder::Layout(_))) && props.is_empty() {
                return None;
            }
            let (linked, current) = bind_state(d, &props, &at.get_value(), b)?;
            let label = linked
                .as_ref()
                .and_then(|n| props.iter().find(|p| &p.name == n))
                .map(|p| p.label().to_string());
            Some((linked, current, label))
        })
    };
    Some(
        view! {
            {move || state().map(|(linked, current, label)| {
                let on = linked.is_some();
                let tip = match &label {
                    Some(l) => ui.now("component-bound-to", &[("label", l.clone())]),
                    None => ui.now("component-bind", &[]),
                };
                view! {
                    <button type="button" class="icon-btn icon-btn--small bind-btn" class:is-on=on
                        data-bind=b.key() data-bound=linked.clone().unwrap_or_default()
                        title=tip.clone() aria-label=tip
                        on:click=move |e| bind_menu(ui, ed, holder, at, b, &e, (linked.clone(), current.clone()))>
                        {if on { icons::component() } else { icons::prop_dot() }}
                    </button>
                }
            })}
        }
        .into_any(),
    )
}

/// Liaisons de l'élément sélectionné qui n'ont pas de champ dans le panneau : son texte (modifié
/// dans le canevas) et sa visibilité. Image, lien et icône portent leur bouton à côté de leur
/// champ ([`bind_button`]). Dans un gabarit, seulement quand il a des variables.
pub fn bindings(ui: Ui, ed: Ed, holder: Holder, at: Target) -> AnyView {
    let holder = StoredValue::new(holder);
    let at = StoredValue::new(at);
    let rows = move || {
        ed.doc.with(|d| {
            let d = d.as_ref()?;
            let props = holder.with_value(|h| h.visible(d))?;
            let at = at.get_value();
            let v: Vec<(Bind, Option<String>, String, Option<String>)> =
                [Bind::Text, Bind::Visible]
                    .into_iter()
                    .filter_map(|b| {
                        let (linked, current) = bind_state(d, &props, &at, b)?;
                        let label = linked
                            .as_ref()
                            .and_then(|n| props.iter().find(|p| &p.name == n))
                            .map(|p| p.label().to_string());
                        Some((b, linked, current, label))
                    })
                    .collect();
            let layout = holder.with_value(|h| matches!(h, Holder::Layout(_)));
            (!layout || !props.is_empty() || v.iter().any(|(_, l, _, _)| l.is_some())).then_some(v)
        })
    };
    view! {
        {move || rows().filter(|v| !v.is_empty()).map(|v| view! {
            <div class="insp-group component-bindings">
                {v.into_iter().map(|(b, linked, current, label)| {
                    let name = linked.clone();
                    view! {
                        <div class="view-line bind-row" data-bind=b.key()>
                            <span class="muted">{ui.t(b.key())}</span>
                            <span class="bind-value">
                                {label.map(|l| view! { <span class="prop-chip">{icons::component()}<span>{l}</span></span> })}
                                <button type="button" class="icon-btn icon-btn--small bind-btn" class:is-on=linked.is_some()
                                    data-bind=b.key() data-bound=name.clone().unwrap_or_default()
                                    title=ui.t("component-bind") aria-label=ui.t("component-bind")
                                    on:click=move |e| bind_menu(ui, ed, holder, at, b, &e, (name.clone(), current.clone()))>
                                    {if linked.is_some() { icons::component() } else { icons::prop_dot() }}
                                </button>
                            </span>
                        </div>
                    }
                }).collect_view()}
            </div>
        })}
    }
    .into_any()
}

/// Ouvre un composant depuis le panneau dans le canevas des composants (même s'il n'est utilisé
/// sur aucune page).
pub fn edit_component(ui: Ui, ed: Ed, id: &str) {
    let first = ed
        .doc
        .with_untracked(|d| first_id(&d.as_ref()?.component(id)?.body));
    match first {
        Some(f) => ed.open_comp(id, Some(f)),
        None => ui.fail(revel_protocol::Failure::new("component-empty")),
    }
}

// ------------------------------------------------------------------ variantes (comme Framer)

/// Nom d'une variante (`None` : principale).
pub fn variant_label(ui: Ui, v: Option<&Variant>) -> String {
    match v {
        None => ui.now("variant-primary", &[]),
        Some(v) => v
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| match v.kind {
                VariantKind::Hover => ui.now("variant-hover", &[]),
                VariantKind::Pressed => ui.now("variant-pressed", &[]),
                VariantKind::Custom => crate::names::humanize(&v.id),
            }),
    }
}

/// Variante personnalisée prise au clic sur le composant (D-121) : interaction de l'élément
/// racine `click class:is-<variante> self` (un clic la prend, le suivant revient à la principale).
pub fn variant_on_click(c: &Component, vid: &str) -> bool {
    root_rules(c).iter().any(|r| is_click_rule(r, vid))
}

fn is_click_rule(r: &revel_edit::actions::Rule, vid: &str) -> bool {
    use revel_edit::actions::{Act, Trigger};
    r.trigger == Trigger::Click && r.target == "self" && r.act == Act::Class(format!("is-{vid}"))
}

fn root_rules(c: &Component) -> Vec<revel_edit::actions::Rule> {
    let Some(Node::Element { attrs, .. }) = c.root() else {
        return Vec::new();
    };
    attrs
        .iter()
        .find(|a| a.name == "data-revel-on")
        .and_then(|a| a.value.as_deref())
        .and_then(revel_edit::actions::parse)
        .unwrap_or_default()
}

/// Commande qui ajoute ou retire le passage à la variante au clic sur l'élément racine.
fn click_command(c: &Component, vid: &str, on: bool) -> Option<Command> {
    use revel_edit::actions::{Act, Rule, Trigger};
    let Some(Node::Element { id: Some(root), .. }) = c.root() else {
        return None;
    };
    let mut rules: Vec<Rule> = root_rules(c)
        .into_iter()
        .filter(|r| !is_click_rule(r, vid))
        .collect();
    if on {
        rules.push(Rule {
            trigger: Trigger::Click,
            act: Act::Class(format!("is-{vid}")),
            target: "self".into(),
        });
    }
    let at = Target {
        owner: Owner::Component(c.id.clone()),
        node: root.clone(),
    };
    Some(if rules.is_empty() {
        Command::RemoveAttr {
            at,
            name: "data-revel-on".into(),
        }
    } else {
        Command::SetAttr {
            at,
            name: "data-revel-on".into(),
            value: Some(revel_edit::actions::write(&rules)),
        }
    })
}

/// Passage à une variante au clic : ajouté ou retiré.
pub fn set_variant_click(ui: Ui, ed: Ed, comp_id: &str, vid: &str, on: bool) {
    let Some(c) = component(ed, comp_id) else {
        return;
    };
    if let Some(cmd) = click_command(&c, vid, on) {
        ed.apply(ui, cmd, false);
    }
}

/// Commande qui donne une classe à l'élément racine du composant s'il n'en a pas (les styles
/// des variantes s'écrivent à partir d'elle) : d'après le nom du composant.
fn root_class_command(d: &revel_model::Site, c: &Component) -> Option<Command> {
    if c.root_class().is_some() {
        return None;
    }
    let Some(Node::Element {
        id: Some(root),
        attrs,
        ..
    }) = c.root()
    else {
        return None;
    };
    let base = revel_model::slugify(&c.label());
    let base = if base.is_empty() {
        "composant".to_string()
    } else {
        base
    };
    let taken: std::collections::HashSet<String> = d
        .style_rules()
        .flat_map(|r| {
            r.selector
                .split(|ch: char| !(ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.'))
                .flat_map(|p| p.split('.').skip(1).map(str::to_string).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .collect();
    let class = (1..)
        .map(|i| {
            if i == 1 {
                base.clone()
            } else {
                format!("{base}-{i}")
            }
        })
        .find(|c| !taken.contains(c))
        .unwrap_or(base);
    let cur = attrs
        .iter()
        .find(|a| a.name == "class")
        .and_then(|a| a.value.clone())
        .unwrap_or_default();
    Some(Command::SetAttr {
        at: Target {
            owner: Owner::Component(c.id.clone()),
            node: root.clone(),
        },
        name: "class".into(),
        value: Some(format!("{class} {cur}").trim().to_string()),
    })
}

/// Ajoute une variante au composant ouvert et l'affiche ; geste déjà présent : affiché.
pub fn add_variant(ui: Ui, ed: Ed, comp_id: &str, kind: VariantKind, click: bool) {
    let Some(c) = component(ed, comp_id) else {
        return;
    };
    let fixed = match kind {
        VariantKind::Hover => Some("survol"),
        VariantKind::Pressed => Some("appui"),
        VariantKind::Custom => None,
    };
    if let Some(v) = c
        .variants
        .iter()
        .find(|v| v.kind == kind && kind != VariantKind::Custom)
    {
        ed.variant.set(v.id.clone());
        return;
    }
    let comp_id = comp_id.to_string();
    let create = move |name: Option<String>, id: String| {
        let Some(c) = component(ed, &comp_id) else {
            return;
        };
        let mut commands: Vec<Command> = ed
            .doc
            .with_untracked(|d| d.as_ref().and_then(|d| root_class_command(d, &c)))
            .into_iter()
            .collect();
        let mut variants = c.variants.clone();
        variants.push(Variant {
            id: id.clone(),
            name,
            kind,
        });
        commands.push(Command::SetComponentVariants {
            id: comp_id.clone(),
            variants,
        });
        if click && let Some(cmd) = click_command(&c, &id, true) {
            commands.push(cmd);
        }
        ed.apply(ui, Command::Batch { commands }, false);
        ed.variant.set(id);
    };
    match fixed {
        Some(id) => create(None, id.to_string()),
        None => {
            let taken: Vec<String> = c.variants.iter().map(|v| v.id.clone()).collect();
            let n = c
                .variants
                .iter()
                .filter(|v| v.kind == VariantKind::Custom)
                .count()
                + 1;
            ui.ask.set(Some(crate::dialog::Ask::input(
                ui.now("variant-new", &[]),
                ui.now("variant-name", &[]),
                ui.now("variant-default-name", &[("n", n.to_string())]),
                ui.now("variant-create", &[]),
                move |name: String| {
                    let base = revel_model::slugify(&name);
                    let base = if revel_model::valid_variant_id(&base) {
                        base
                    } else {
                        "variante".into()
                    };
                    let id = (1..)
                        .map(|i| {
                            if i == 1 {
                                base.clone()
                            } else {
                                format!("{base}-{i}")
                            }
                        })
                        .find(|x| !taken.contains(x) && !matches!(x.as_str(), "survol" | "appui"))
                        .unwrap_or(base);
                    create(Some(name.trim().to_string()).filter(|n| !n.is_empty()), id)
                },
            )));
        }
    }
}

/// Renomme une variante (nom affiché ; l'identifiant et la classe ne changent pas).
pub fn rename_variant(ui: Ui, ed: Ed, comp_id: &str, vid: &str) {
    let Some(c) = component(ed, comp_id) else {
        return;
    };
    let Some(v) = c.variant(vid).cloned() else {
        return;
    };
    let comp_id = comp_id.to_string();
    let vid = vid.to_string();
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("variant-rename", &[]),
        ui.now("variant-name", &[]),
        variant_label(ui, Some(&v)),
        ui.now("variant-rename", &[]),
        move |name: String| {
            let Some(c) = component(ed, &comp_id) else {
                return;
            };
            let variants = c
                .variants
                .iter()
                .map(|x| {
                    let mut x = x.clone();
                    if x.id == vid {
                        x.name = Some(name.trim().to_string()).filter(|n| !n.is_empty());
                    }
                    x
                })
                .collect();
            ed.apply(
                ui,
                Command::SetComponentVariants {
                    id: comp_id.clone(),
                    variants,
                },
                false,
            );
        },
    )));
}

/// Retire une variante : ses styles (variante personnalisée) et son choix dans les utilisations
/// avec elle ; les styles de survol et d'appui restent (ils peuvent venir du site).
pub fn remove_variant(ui: Ui, ed: Ed, comp_id: &str, vid: &str) {
    let Some(c) = component(ed, comp_id) else {
        return;
    };
    let Some(v) = c.variant(vid).cloned() else {
        return;
    };
    let comp_id = comp_id.to_string();
    let vid = vid.to_string();
    let name = variant_label(ui, Some(&v));
    ui.ask.set(Some(
        crate::dialog::Ask::confirm(
            ui.now("variant-remove-title", &[("name", name)]),
            None,
            ui.now("action-remove", &[]),
            move || {
                let Some(c) = component(ed, &comp_id) else {
                    return;
                };
                let mut commands = vec![Command::SetComponentVariants {
                    id: comp_id.clone(),
                    variants: c.variants.iter().filter(|x| x.id != vid).cloned().collect(),
                }];
                ed.doc.with_untracked(|d| {
                    let Some(d) = d.as_ref() else { return };
                    if v.kind == VariantKind::Custom
                        && let Some(root) = c.root_class()
                    {
                        let prefix = format!(".{root}{}", v.suffix());
                        for r in d.style_rules() {
                            let hit = r.selector == prefix
                                || r.selector.starts_with(&format!("{prefix} "))
                                || r.selector.starts_with(&format!("{prefix}:"));
                            if hit && let Some(id) = &r.id {
                                commands.push(Command::RemoveRule { rule: id.clone() });
                            }
                        }
                    }
                    for (kind, owner, body) in d.owners() {
                        revel_model::ids::walk(body, &mut |n| {
                            if let Node::Use {
                                id: Some(uid),
                                component,
                                variant: Some(x),
                                ..
                            } = n
                                && *component == comp_id
                                && *x == vid
                            {
                                let owner = match kind {
                                    revel_model::ids::OwnerKind::Page => {
                                        Owner::Page(owner.to_string())
                                    }
                                    revel_model::ids::OwnerKind::Layout => {
                                        Owner::Layout(owner.to_string())
                                    }
                                    revel_model::ids::OwnerKind::Component => {
                                        Owner::Component(owner.to_string())
                                    }
                                };
                                commands.push(Command::SetUseVariant {
                                    at: Target {
                                        owner,
                                        node: uid.clone(),
                                    },
                                    variant: None,
                                });
                            }
                        });
                    }
                });
                if ed.variant.get_untracked() == vid {
                    ed.variant.set(String::new());
                }
                ed.apply(ui, Command::Batch { commands }, false);
            },
        )
        .danger(),
    ));
}

/// Renomme un composant (nom affiché ; l'identifiant ne change pas).
pub fn rename(ui: Ui, ed: Ed, id: &str) {
    let Some(c) = component(ed, id) else { return };
    let id = id.to_string();
    ui.ask.set(Some(crate::dialog::Ask::input(
        ui.now("component-rename", &[]),
        ui.now("component-make-name", &[]),
        crate::names::component_name(&c),
        ui.now("component-rename", &[]),
        move |name| {
            ed.apply(
                ui,
                Command::SetComponentName {
                    id: id.clone(),
                    name: Some(name),
                },
                false,
            )
        },
    )));
}

/// Supprime un composant inutilisé (confirmation), retiré aussi de son dossier.
pub fn delete(ui: Ui, ed: Ed, id: &str) {
    let Some(c) = component(ed, id) else { return };
    let id = id.to_string();
    ui.ask.set(Some(
        crate::dialog::Ask::confirm(
            ui.now(
                "component-delete-title",
                &[("name", crate::names::component_name(&c))],
            ),
            None,
            ui.now("component-delete", &[]),
            move || {
                let mut commands = vec![Command::RemoveComponent { id: id.clone() }];
                if let Some(shelf) = ed.doc.with_untracked(|d| {
                    let s = &d.as_ref()?.library.components;
                    s.items.contains_key(&id).then(|| {
                        let mut s = s.clone();
                        s.items.remove(&id);
                        s
                    })
                }) {
                    commands.push(Command::SetShelf {
                        kind: revel_edit::ShelfKind::Components,
                        shelf,
                    });
                }
                ed.apply(ui, Command::Batch { commands }, false);
            },
        )
        .danger(),
    ));
}

/// Transitions proposées pour les variantes de survol et d'appui : durée, clé du libellé.
const TRANSITIONS: &[(&str, &str)] = &[
    ("", "variant-transition-none"),
    ("0.15s", "variant-transition-fast"),
    ("0.3s", "variant-transition-smooth"),
    ("0.6s", "variant-transition-slow"),
];

/// Transition des variantes de survol et d'appui (comme les transitions de Framer) : règle
/// `.racine, .racine *` de la feuille du site (propriétés visuelles, durée, courbe douce).
#[component]
pub fn TransitionSelect(id: String) -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let id = StoredValue::new(id);
    let selector = Memo::new(move |_| {
        ed.doc.with(|d| {
            let r = d.as_ref()?.component(&id.get_value())?.root_class()?;
            Some(format!(".{r}, .{r} *"))
        })
    });
    let rule = Memo::new(move |_| {
        let sel = selector.get()?;
        ed.doc
            .with(|d| crate::styles::find_rule(d.as_ref()?, &sel, &None))
    });
    let current = move || {
        rule.with(|r| {
            r.as_ref()
                .and_then(|r| revel_style::values::read(&r.decls, "transition-duration"))
                .and_then(|f| f.value)
        })
        .unwrap_or_default()
    };
    let change = move |v: String| {
        let Some(sel) = selector.get_untracked() else {
            return;
        };
        let on = !v.is_empty();
        let list = vec![
            (
                "transition-property".to_string(),
                on.then(|| {
                    "color, background-color, border-color, opacity, transform, box-shadow, filter"
                        .to_string()
                }),
            ),
            ("transition-duration".to_string(), on.then(|| v.clone())),
            (
                "transition-timing-function".to_string(),
                on.then(|| "cubic-bezier(.2, .8, .2, 1)".to_string()),
            ),
        ];
        if let Some(cmd) = crate::styles::style_command(rule.get_untracked(), sel, None, list) {
            ed.apply(ui, cmd, false);
        }
    };
    view! {
        <span class="select select--small comp-transition" title=ui.t("variant-transition-help")
            class:is-disabled=move || selector.with(Option::is_none)>
            <select class="input" aria-label=ui.t("variant-transition") prop:disabled=move || selector.with(Option::is_none)
                on:change=move |e| change(event_target_value(&e))>
                {TRANSITIONS.iter().map(|(v, k)| {
                    let v: &'static str = v;
                    view! { <option value=v selected=move || current() == v>{ui.t(k)}</option> }
                }).collect_view()}
            </select>
        </span>
    }
}

/// Liste écrite à la main (`["a", 'b']`) : ses textes ; `None` si c'est une expression (champ
/// relié, filtre…).
pub(crate) fn list_literal(expr: &str) -> Option<Vec<String>> {
    let inner = expr.trim().strip_prefix('[')?.strip_suffix(']')?.trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    let (mut cur, mut quote_ch, mut esc) = (String::new(), None::<char>, false);
    for c in inner.chars() {
        if let Some(q) = quote_ch {
            cur.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == q {
                quote_ch = None;
            }
        } else if c == ',' {
            out.push(literal(cur.trim())?);
            cur.clear();
        } else {
            if c == '"' || c == '\'' {
                quote_ch = Some(c);
            }
            cur.push(c);
        }
    }
    out.push(literal(cur.trim())?);
    Some(out)
}

#[cfg(test)]
mod list_tests {
    use super::list_literal;

    #[test]
    fn listes_ecrites() {
        assert_eq!(
            list_literal(r#"["a", 'b, c', "d\"e"]"#),
            Some(vec!["a".into(), "b, c".into(), "d\"e".into()])
        );
        assert_eq!(list_literal("[]"), Some(vec![]));
        assert_eq!(list_literal("item.data.tags"), None);
        assert_eq!(list_literal("[x, 'y']"), None);
    }
}

/// Contenus d'une collection : identifiant (champ `slug`, sinon nom du fichier) et titre (D-174).
pub(crate) fn entries_of(
    cols: RwSignal<Vec<revel_protocol::CmsCollection>>,
    collection: &str,
) -> Vec<(String, String)> {
    cols.with(|cs| {
        cs.iter()
            .find(|c| c.id == collection)
            .map(|c| {
                c.entries
                    .iter()
                    .map(|e| {
                        let text = |k: &str| {
                            e.data
                                .get(k)
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .map(str::to_string)
                        };
                        let slug = text("slug").unwrap_or_else(|| {
                            e.file
                                .rsplit_once('.')
                                .map_or(e.file.as_str(), |(a, _)| a)
                                .to_string()
                        });
                        let title = text("title")
                            .or_else(|| text("name"))
                            .unwrap_or_else(|| slug.clone());
                        (slug, title)
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}
