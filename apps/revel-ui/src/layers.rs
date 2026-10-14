//! Calques (étape 2.4) : arbre de la page affichée — gabarits emboîtés, page, composants utilisés,
//! blocs logiques — avec sélection synchronisée avec le canevas, glisser-déposer, dupliquer,
//! envelopper, supprimer, monter et descendre.

use crate::editor::{Ed, Selection};
use crate::{Ui, icons};
use leptos::prelude::*;
use revel_edit::{Branch, Command, Owner, Parent, Target};
use revel_model::{Node, Site};
use std::collections::HashSet;

/// Nature d'une ligne de l'arbre.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Element {
        tag: String,
        class: String,
    },
    Text(String),
    Expr(String),
    If(String),
    Each {
        item: String,
        expr: String,
    },
    /// Branche « sinon » d'une condition, « si vide » d'une boucle.
    Branch(Branch),
    Use(String),
    Image(String),
    Html,
    Var(String),
    /// Emplacement du contenu dans un gabarit (contenu Markdown pour un contenu de collection).
    Slot {
        markdown: bool,
    },
}

/// Ligne de l'arbre des calques.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Clé unique (chemin dans l'arbre affiché).
    pub key: String,
    /// Clés des lignes parentes.
    pub ancestors: Vec<String>,
    pub depth: usize,
    pub id: Option<String>,
    pub owner: Owner,
    /// Liste qui contient le nœud, et sa position.
    pub parent: Parent,
    pub index: usize,
    pub kind: Kind,
    /// A des lignes enfants.
    pub has_children: bool,
    /// Peut recevoir des enfants (glisser-déposer « dans ») : branche et nombre d'enfants.
    pub inside: Option<(Branch, usize)>,
}

fn class_of(attrs: &[revel_model::Attr]) -> String {
    attrs
        .iter()
        .find(|a| a.name == "class")
        .and_then(|a| a.value.as_deref())
        .filter(|v| !v.contains("{{") && !v.contains("{%"))
        .and_then(|v| v.split_whitespace().next())
        .map(str::to_string)
        .unwrap_or_default()
}

fn short(s: &str, n: usize) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() > n {
        format!("{}…", t.chars().take(n).collect::<String>())
    } else {
        t
    }
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

struct Builder<'a> {
    doc: &'a Site,
    /// Corps à insérer à la place de chaque `Slot`, du gabarit extérieur vers la page.
    inner: Vec<(Owner, &'a [Node])>,
    markdown: bool,
    rows: Vec<Row>,
    /// Composants en cours de parcours (garde contre un composant qui s'utilise lui-même).
    stack: Vec<String>,
}

impl<'a> Builder<'a> {
    fn list(
        &mut self,
        nodes: &'a [Node],
        owner: &Owner,
        parent: Parent,
        key: &str,
        ancestors: &[String],
        slot_level: usize,
    ) {
        for (i, n) in nodes.iter().enumerate() {
            if matches!(n, Node::Text { text } if text.trim().is_empty())
                || matches!(n, Node::Html { html } if html.trim().is_empty())
            {
                continue;
            }
            let k = format!("{key}/{i}");
            let depth = ancestors.len();
            let id = revel_model::ids::node_id(n).map(str::to_string);
            let kind = match n {
                Node::Element { tag, attrs, .. } => Kind::Element {
                    tag: tag.clone(),
                    class: class_of(attrs),
                },
                Node::Text { text } => Kind::Text(short(text, 40)),
                Node::Out { expr } => Kind::Expr(short(expr, 40)),
                Node::Html { .. } => Kind::Html,
                Node::If { cond, .. } => Kind::If(short(cond, 40)),
                Node::Each { item, expr, .. } => Kind::Each {
                    item: item.clone(),
                    expr: short(expr, 32),
                },
                Node::Set { name, .. } | Node::SetBlock { name, .. } => Kind::Var(name.clone()),
                Node::Slot => Kind::Slot {
                    markdown: self.markdown && slot_level + 1 >= self.inner.len(),
                },
                Node::Use { component, .. } => Kind::Use(component.clone()),
                Node::Image(img) => Kind::Image(short(&img.src, 32)),
            };
            let inside = match n {
                Node::Element { tag, children, .. }
                    if !VOID.contains(&tag.to_ascii_lowercase().as_str()) =>
                {
                    Some((Branch::Children, children.len()))
                }
                Node::Each { children, .. } => Some((Branch::Children, children.len())),
                Node::If { then, .. } => Some((Branch::Then, then.len())),
                _ => None,
            };
            let at = self.rows.len();
            self.rows.push(Row {
                key: k.clone(),
                ancestors: ancestors.to_vec(),
                depth,
                id: id.clone(),
                owner: owner.clone(),
                parent: parent.clone(),
                index: i,
                kind,
                has_children: false,
                inside: inside.filter(|_| id.is_some()),
            });
            let mut anc = ancestors.to_vec();
            anc.push(k.clone());
            let before = self.rows.len();
            let of = |b: Branch| Parent {
                owner: owner.clone(),
                node: id.clone(),
                branch: b,
            };
            match n {
                Node::Element { children, .. } | Node::SetBlock { children, .. } => {
                    self.list(children, owner, of(Branch::Children), &k, &anc, slot_level);
                }
                Node::If {
                    then, otherwise, ..
                } => {
                    self.list(then, owner, of(Branch::Then), &k, &anc, slot_level);
                    if !otherwise.is_empty() {
                        self.branch(
                            Branch::Otherwise,
                            otherwise,
                            owner,
                            &id,
                            &k,
                            &anc,
                            slot_level,
                        );
                    }
                }
                Node::Each {
                    children, empty, ..
                } => {
                    self.list(children, owner, of(Branch::Children), &k, &anc, slot_level);
                    if !empty.is_empty() {
                        self.branch(Branch::Empty, empty, owner, &id, &k, &anc, slot_level);
                    }
                }
                Node::Use { component, .. } => {
                    if let Some(c) = self.doc.component(component)
                        && !self.stack.contains(component)
                    {
                        self.stack.push(component.clone());
                        let o = Owner::Component(c.id.clone());
                        self.list(&c.body, &o, Parent::root(o.clone()), &k, &anc, slot_level);
                        self.stack.pop();
                    }
                }
                Node::Slot => {
                    if let Some((o, body)) = self.inner.get(slot_level + 1).cloned() {
                        self.list(body, &o, Parent::root(o.clone()), &k, &anc, slot_level + 1);
                    }
                }
                _ => {}
            }
            self.rows[at].has_children = self.rows.len() > before;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn branch(
        &mut self,
        b: Branch,
        nodes: &'a [Node],
        owner: &Owner,
        id: &Option<String>,
        key: &str,
        ancestors: &[String],
        slot_level: usize,
    ) {
        let k = format!("{key}/{b:?}");
        let parent = Parent {
            owner: owner.clone(),
            node: id.clone(),
            branch: b,
        };
        let at = self.rows.len();
        self.rows.push(Row {
            key: k.clone(),
            ancestors: ancestors.to_vec(),
            depth: ancestors.len(),
            id: None,
            owner: owner.clone(),
            parent: parent.clone(),
            index: 0,
            kind: Kind::Branch(b),
            has_children: false,
            inside: id.as_ref().map(|_| (b, nodes.len())),
        });
        let mut anc = ancestors.to_vec();
        anc.push(k.clone());
        let before = self.rows.len();
        self.list(nodes, owner, parent, &k, &anc, slot_level);
        self.rows[at].has_children = self.rows.len() > before;
    }
}

/// Toutes les lignes de l'arbre de la page (gabarits emboîtés, page, composants).
pub fn rows(doc: &Site, page: &revel_protocol::EditorPageInfo) -> Vec<Row> {
    let mut chain: Vec<&revel_model::Layout> = Vec::new();
    let mut cur = page.layout.clone();
    while let Some(id) = cur {
        match doc.layout(&id) {
            Some(l) if !chain.iter().any(|c| c.id == l.id) => {
                chain.push(l);
                cur = l.parent.clone();
            }
            _ => break,
        }
    }
    // Du gabarit le plus extérieur vers la page.
    let mut inner: Vec<(Owner, &[Node])> = chain
        .iter()
        .rev()
        .map(|l| (Owner::Layout(l.id.clone()), l.body.as_slice()))
        .collect();
    let fixed = page
        .page
        .as_ref()
        .and_then(|id| doc.pages.iter().find(|p| &p.id == id));
    if let Some(p) = fixed {
        inner.push((Owner::Page(p.id.clone()), p.body.as_slice()));
    }
    let mut b = Builder {
        doc,
        inner: inner.clone(),
        markdown: fixed.is_none(),
        rows: Vec::new(),
        stack: Vec::new(),
    };
    if let Some((o, body)) = inner.first().cloned() {
        b.list(body, &o, Parent::root(o.clone()), "", &[], 0);
    }
    b.rows
}

/// Commande de déplacement d'une ligne vers une autre (`zone` : avant, après, dans).
pub fn move_command(src: &Row, dst: &Row, zone: Zone) -> Option<Command> {
    let id = src.id.clone()?;
    if dst.key == src.key || dst.ancestors.contains(&src.key) {
        return None;
    }
    let (to, mut index) = match zone {
        Zone::Before => (dst.parent.clone(), dst.index),
        Zone::After => (dst.parent.clone(), dst.index + 1),
        Zone::Inside => {
            let (branch, n) = dst.inside?;
            let to = match &dst.id {
                Some(i) => Parent {
                    owner: dst.owner.clone(),
                    node: Some(i.clone()),
                    branch,
                },
                // Ligne « sinon » / « si vide » : sa liste.
                None => dst.parent.clone(),
            };
            (to, n)
        }
    };
    // Seulement à l'intérieur d'une même page, d'un même gabarit ou d'un même composant.
    if to.owner != src.owner {
        return None;
    }
    if to == src.parent && src.index < index {
        index -= 1;
    }
    if to == src.parent && index == src.index {
        return None;
    }
    Some(Command::Move {
        at: Target {
            owner: src.owner.clone(),
            node: id,
        },
        to,
        index,
    })
}

/// Monter (`-1`) ou descendre (`+1`) parmi les frères visibles.
pub fn step_command(all: &[Row], row: &Row, dir: i32) -> Option<Command> {
    let siblings: Vec<&Row> = all
        .iter()
        .filter(|r| r.parent == row.parent && r.ancestors == row.ancestors)
        .collect();
    let pos = siblings.iter().position(|r| r.key == row.key)?;
    let other = if dir < 0 {
        siblings.get(pos.checked_sub(1)?)?
    } else {
        siblings.get(pos + 1)?
    };
    move_command(row, other, if dir < 0 { Zone::Before } else { Zone::After })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Before,
    After,
    Inside,
}

fn label(ui: Ui, k: &Kind) -> String {
    match k {
        Kind::Element { tag, class } if class.is_empty() => tag.clone(),
        Kind::Element { tag, class } => format!("{tag}.{class}"),
        Kind::Text(t) => format!("« {t} »"),
        Kind::Expr(e) => format!("{{{{ {e} }}}}"),
        Kind::If(c) => ui.tx("layers-if", &[("cond", c.clone())]),
        Kind::Each { item, expr } => ui.tx(
            "layers-each",
            &[("item", item.clone()), ("list", expr.clone())],
        ),
        Kind::Branch(Branch::Otherwise) => ui.tx("layers-else", &[]),
        Kind::Branch(_) => ui.tx("layers-empty-list", &[]),
        Kind::Use(c) => c.rsplit('/').next().unwrap_or(c).to_string(),
        Kind::Image(src) => ui.tx("layers-image", &[("src", src.clone())]),
        Kind::Html => ui.tx("layers-html", &[]),
        Kind::Var(n) => ui.tx("layers-var", &[("name", n.clone())]),
        Kind::Slot { markdown: true } => ui.tx("layers-markdown", &[]),
        Kind::Slot { markdown: false } => ui.tx("layers-slot", &[]),
    }
}

fn icon(k: &Kind) -> AnyView {
    match k {
        Kind::Element { .. } => icons::box_(),
        Kind::Text(_) => icons::text(),
        Kind::Expr(_) => icons::braces(),
        Kind::If(_) | Kind::Branch(_) => icons::branch(),
        Kind::Each { .. } => icons::repeat(),
        Kind::Use(_) => icons::component(),
        Kind::Image(_) => icons::image(),
        Kind::Html | Kind::Var(_) => icons::code(),
        Kind::Slot { .. } => icons::slot(),
    }
}

#[component]
pub fn Layers() -> impl IntoView {
    let ui = expect_context::<Ui>();
    let ed = expect_context::<Ed>();
    let opened = RwSignal::new(HashSet::<String>::new());
    let closed = RwSignal::new(HashSet::<String>::new());
    let drag = RwSignal::new(None::<Row>);
    let drop_at = RwSignal::new(None::<(String, Zone)>);
    let query = RwSignal::new(String::new());
    // Menu contextuel (clic droit) : position dans la fenêtre.
    let menu = RwSignal::new(None::<(f64, f64)>);

    let all = Memo::new(move |_| {
        let url = ed.page.get();
        ed.doc.with(|d| {
            let Some(d) = d else { return Vec::new() };
            ed.pages.with(|ps| {
                ps.iter()
                    .find(|p| p.url == url)
                    .map(|p| rows(d, p))
                    .unwrap_or_default()
            })
        })
    });
    // Ouverture par défaut : toute la structure visible, sauf l'en-tête du document (`head`), les
    // composants et les niveaux profonds ; le reste s'ouvre à la demande ou à la sélection.
    let default_open = |r: &Row| {
        r.depth < 8
            && !matches!(&r.kind, Kind::Element { tag, .. } if tag.eq_ignore_ascii_case("head"))
            && !matches!(r.kind, Kind::Use(_))
    };
    let is_open = move |r: &Row, op: &HashSet<String>, cl: &HashSet<String>| {
        op.contains(&r.key) || (default_open(r) && !cl.contains(&r.key))
    };
    let visible = Memo::new(move |_| {
        let op = opened.get();
        let cl = closed.get();
        let q = query.get().trim().to_lowercase();
        all.with(|rows| {
            // Recherche : lignes dont le libellé correspond, avec leurs parents (ouverts).
            if !q.is_empty() {
                let hits: Vec<&Row> = rows
                    .iter()
                    .filter(|r| label(ui, &r.kind).to_lowercase().contains(&q))
                    .collect();
                let keep: HashSet<&str> = hits
                    .iter()
                    .flat_map(|r| {
                        r.ancestors
                            .iter()
                            .map(String::as_str)
                            .chain(std::iter::once(r.key.as_str()))
                    })
                    .collect();
                return rows
                    .iter()
                    .filter(|r| keep.contains(r.key.as_str()))
                    .map(|r| (r.clone(), true))
                    .collect::<Vec<_>>();
            }
            let open: HashSet<&str> = rows
                .iter()
                .filter(|r| is_open(r, &op, &cl))
                .map(|r| r.key.as_str())
                .collect();
            rows.iter()
                .filter(|r| r.ancestors.iter().all(|a| open.contains(a.as_str())))
                .map(|r| (r.clone(), open.contains(r.key.as_str())))
                .collect::<Vec<_>>()
        })
    });
    // Sélection venue du canevas : ouvrir le chemin jusqu'à l'élément.
    Effect::new(move |_| {
        let Some(sel) = ed.selected.get() else {
            return;
        };
        let anc = all.with_untracked(|rows| {
            rows.iter()
                .find(|r| r.id.as_deref() == Some(sel.id.as_str()))
                .map(|r| r.ancestors.clone())
        });
        if let Some(anc) = anc {
            opened.update(|o| o.extend(anc.iter().cloned()));
            closed.update(|c| {
                for a in &anc {
                    c.remove(a);
                }
            });
        }
        // Ligne sélectionnée rendue visible dans l'arbre (après l'affichage des lignes ouvertes).
        request_animation_frame(|| {
            request_animation_frame(|| {
                if let Ok(Some(el)) = document().query_selector(".layers .layer.is-selected") {
                    let opts = web_sys::ScrollIntoViewOptions::new();
                    opts.set_block(web_sys::ScrollLogicalPosition::Center);
                    el.scroll_into_view_with_scroll_into_view_options(&opts);
                }
            });
        });
    });

    let select_row = move |r: &Row| {
        // Ligne sans identifiant (texte, expression) : son élément parent.
        let id = r.id.clone().or_else(|| r.parent.node.clone());
        let Some(id) = id else { return };
        let label = label(ui, &r.kind);
        ed.selected.set(Some(Selection {
            id: id.clone(),
            tag: match &r.kind {
                Kind::Element { tag, .. } => tag.clone(),
                _ => String::new(),
            },
            label,
        }));
        ed.post(serde_json::json!({ "type": "select", "id": id, "reveal": true }));
    };
    let selected_row = move || {
        let sel = ed.selected.get()?;
        all.with(|rows| {
            rows.iter()
                .find(|r| r.id.as_deref() == Some(sel.id.as_str()))
                .cloned()
        })
    };
    let target = move || {
        selected_row().and_then(|r| {
            Some(Target {
                owner: r.owner.clone(),
                node: r.id.clone()?,
            })
        })
    };
    let act = move |cmd: Option<Command>| {
        if let Some(c) = cmd {
            ed.apply(ui, c, false);
        }
    };

    // Parent de la ligne sélectionnée (élément qui la contient).
    let parent_row = move || {
        let r = selected_row()?;
        all.with(|rows| {
            r.ancestors
                .iter()
                .rev()
                .find_map(|a| rows.iter().find(|x| &x.key == a && x.id.is_some()).cloned())
        })
    };
    // Clavier : ↑ ↓ ligne précédente ou suivante, ← replier (ou parent), → déplier.
    let on_key = move |e: leptos::ev::KeyboardEvent| {
        let key = e.key();
        if !matches!(
            key.as_str(),
            "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"
        ) {
            return;
        }
        e.prevent_default();
        let rows: Vec<(Row, bool)> = visible.get_untracked();
        let cur = selected_row();
        let pos = cur
            .as_ref()
            .and_then(|c| rows.iter().position(|(r, _)| r.key == c.key));
        match key.as_str() {
            "ArrowUp" | "ArrowDown" => {
                let up = key == "ArrowUp";
                let start = pos.map(|p| p as i64).unwrap_or(-1);
                let mut i = start;
                loop {
                    i += if up { -1 } else { 1 };
                    if i < 0 || i as usize >= rows.len() {
                        break;
                    }
                    if rows[i as usize].0.id.is_some() {
                        select_row(&rows[i as usize].0);
                        break;
                    }
                }
            }
            "ArrowLeft" => {
                if let Some((r, open)) = pos.map(|p| rows[p].clone()) {
                    if open && r.has_children {
                        opened.update(|o| {
                            o.remove(&r.key);
                        });
                        closed.update(|c| {
                            c.insert(r.key.clone());
                        });
                    } else if let Some(p) = parent_row() {
                        select_row(&p);
                    }
                }
            }
            _ => {
                if let Some((r, open)) = pos.map(|p| rows[p].clone())
                    && !open
                    && r.has_children
                {
                    closed.update(|c| {
                        c.remove(&r.key);
                    });
                    opened.update(|o| {
                        o.insert(r.key.clone());
                    });
                }
            }
        }
    };
    let collapse_all = move |_| {
        let keys: HashSet<String> = all.with_untracked(|rows| {
            rows.iter()
                .filter(|r| r.has_children && r.depth > 0)
                .map(|r| r.key.clone())
                .collect()
        });
        opened.set(HashSet::new());
        closed.set(keys);
    };
    let on_outside =
        window_event_listener(leptos::ev::pointerdown, move |e: web_sys::PointerEvent| {
            if menu.get_untracked().is_none() {
                return;
            }
            let inside = e
                .target()
                .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
                .and_then(|el| el.closest(".context-menu").ok().flatten())
                .is_some();
            if !inside {
                menu.set(None);
            }
        });
    // Clic dans le canevas (autre document) : la fenêtre perd le focus.
    let on_blur = window_event_listener(leptos::ev::blur, move |_| menu.set(None));
    on_cleanup(move || {
        on_outside.remove();
        on_blur.remove();
    });
    let menu_item = move |key: &'static str,
                          shortcut: &'static str,
                          icon: fn() -> AnyView,
                          run: Box<dyn Fn() + Send + Sync>| {
        view! {
            <button class="menu-item context-item" role="menuitem" on:click=move |_| { menu.set(None); run(); }>
                <span class="menu-icon">{icon()}</span>
                <span>{ui.t(key)}</span>
                {(!shortcut.is_empty()).then(|| view! { <kbd>{ui.t(shortcut)}</kbd> })}
            </button>
        }
    };

    view! {
        <aside class="layers" aria-label=ui.t("layers-title")>
            <label class="panel-search">
                {icons::search()}
                <input type="search" placeholder=ui.t("layers-search") aria-label=ui.t("layers-search")
                    prop:value=move || query.get() on:input=move |e| query.set(event_target_value(&e))/>
            </label>
            <div class="layers-tree" role="tree" tabindex="0" on:keydown=on_key>
                <For each=move || visible.get()
                    key=|(r, open)| (r.key.clone(), format!("{:?}{}{}{:?}", r.kind, r.has_children, open, r.id))
                    let:item>
                    {
                        let (r, open) = item;
                        let r = StoredValue::new(r);
                        let key = r.with_value(|r| r.key.clone());
                        let k_sel = key.clone();
                        let k_drop = key.clone();
                        let id = r.with_value(|r| r.id.clone());
                        let id_hover = id.clone();
                        let depth = r.with_value(|r| r.depth);
                        let has_children = r.with_value(|r| r.has_children);
                        let kind = r.with_value(|r| r.kind.clone());
                        let owner_badge = r.with_value(|r| match (&r.kind, &r.owner) {
                            (Kind::Use(_), _) => Some("layers-badge-component"),
                            _ => None,
                        });
                        let toggle = move |e: leptos::ev::MouseEvent| {
                            e.stop_propagation();
                            let k = r.with_value(|r| r.key.clone());
                            if open {
                                opened.update(|o| { o.remove(&k); });
                                closed.update(|c| { c.insert(k); });
                            } else {
                                closed.update(|c| { c.remove(&k); });
                                opened.update(|o| { o.insert(k); });
                            }
                        };
                        let is_selected = move || {
                            let sel = ed.selected.get().map(|s| s.id);
                            sel.is_some() && sel == id.clone() && all.with(|rows| {
                                rows.iter().find(|x| x.id == sel).map(|x| x.key == k_sel).unwrap_or(false)
                            })
                        };
                        let zone_class = move |z: Zone| {
                            let k = k_drop.clone();
                            move || drop_at.get().is_some_and(|(dk, dz)| dk == k && dz == z)
                        };
                        let draggable = r.with_value(|r| r.id.is_some()).to_string();
                        view! {
                            <div class="layer" role="treeitem" class:is-selected=is_selected
                                class:is-logic=matches!(kind, Kind::If(_) | Kind::Each { .. } | Kind::Branch(_) | Kind::Var(_))
                                class:is-text=matches!(kind, Kind::Text(_) | Kind::Expr(_) | Kind::Html)
                                class:drop-before=zone_class(Zone::Before)
                                class:drop-after=zone_class(Zone::After)
                                class:drop-inside=zone_class(Zone::Inside)
                                style=format!("--depth:{depth}")
                                data-depth=depth
                                draggable=draggable
                                on:click=move |_| r.with_value(|r| select_row(r))
                                on:contextmenu=move |e: leptos::ev::MouseEvent| {
                                    e.prevent_default();
                                    r.with_value(|r| select_row(r));
                                    menu.set(Some((f64::from(e.client_x()), f64::from(e.client_y()))));
                                }
                                on:mouseenter=move |_| ed.post(serde_json::json!({ "type": "hover", "id": id_hover.clone() }))
                                on:mouseleave=move |_| ed.post(serde_json::json!({ "type": "hover", "id": null }))
                                on:dragstart=move |e: leptos::ev::DragEvent| {
                                    if let Some(dt) = e.data_transfer() {
                                        let _ = dt.set_data("text/plain", &key);
                                        dt.set_effect_allowed("move");
                                    }
                                    drag.set(Some(r.get_value()));
                                }
                                on:dragend=move |_| { drag.set(None); drop_at.set(None); }
                                on:dragover=move |e: leptos::ev::DragEvent| {
                                    let Some(src) = drag.get_untracked() else { return };
                                    let dst = r.get_value();
                                    let rect = e
                                        .current_target()
                                        .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok())
                                        .map(|el| el.get_bounding_client_rect());
                                    let Some(rect) = rect else { return };
                                    let y = (f64::from(e.client_y()) - rect.top()) / rect.height().max(1.0);
                                    let zone = if dst.inside.is_some() && (0.3..0.7).contains(&y) {
                                        Zone::Inside
                                    } else if y < 0.5 {
                                        Zone::Before
                                    } else {
                                        Zone::After
                                    };
                                    if move_command(&src, &dst, zone).is_some() {
                                        e.prevent_default();
                                        drop_at.set(Some((dst.key.clone(), zone)));
                                    } else {
                                        drop_at.set(None);
                                    }
                                }
                                on:drop=move |e: leptos::ev::DragEvent| {
                                    e.prevent_default();
                                    let src = drag.get_untracked();
                                    let zone = drop_at.get_untracked().map(|(_, z)| z);
                                    drag.set(None);
                                    drop_at.set(None);
                                    if let (Some(src), Some(zone)) = (src, zone) {
                                        act(move_command(&src, &r.get_value(), zone));
                                    }
                                }>
                                <span class="layer-guides" aria-hidden="true"></span>
                                <button class="layer-toggle" tabindex="-1" aria-hidden="true"
                                    class:is-hidden=!has_children on:click=toggle>
                                    {if open { icons::chevron_down() } else { icons::chevron_right() }}
                                </button>
                                <span class="layer-icon">{icon(&kind)}</span>
                                <span class="layer-label">{label(ui, &kind)}</span>
                                {owner_badge.map(|k| view! { <span class="layer-badge">{ui.t(k)}</span> })}
                            </div>
                        }
                    }
                </For>
            </div>
            <footer class="panel-foot layers-foot">
                <button class="icon-btn icon-btn--small" title=ui.t("layers-collapse") aria-label=ui.t("layers-collapse")
                    on:click=collapse_all>{icons::collapse()}</button>
                <span class="toolbar-spacer"></span>
                <button class="icon-btn icon-btn--small" title=ui.t("layers-parent") aria-label=ui.t("layers-parent")
                    prop:disabled=move || parent_row().is_none()
                    on:click=move |_| { if let Some(p) = parent_row() { select_row(&p) } }>{icons::parent()}</button>
                <button class="icon-btn icon-btn--small" title=ui.t("layers-up") aria-label=ui.t("layers-up")
                    prop:disabled=move || selected_row().is_none()
                    on:click=move |_| act(selected_row().and_then(|r| all.with_untracked(|a| step_command(a, &r, -1))))>{icons::arrow_up()}</button>
                <button class="icon-btn icon-btn--small" title=ui.t("layers-down") aria-label=ui.t("layers-down")
                    prop:disabled=move || selected_row().is_none()
                    on:click=move |_| act(selected_row().and_then(|r| all.with_untracked(|a| step_command(a, &r, 1))))>{icons::arrow_down()}</button>
                <button class="icon-btn icon-btn--small" title=ui.t("layers-wrap") aria-label=ui.t("layers-wrap")
                    prop:disabled=move || target().is_none()
                    on:click=move |_| act(target().map(|at| Command::Wrap { at, tag: "div".into() }))>{icons::wrap()}</button>
                <button class="icon-btn icon-btn--small" title=ui.t("layers-duplicate") aria-label=ui.t("layers-duplicate")
                    prop:disabled=move || target().is_none()
                    on:click=move |_| act(target().map(|at| Command::Duplicate { at }))>{icons::copy()}</button>
                <button class="icon-btn icon-btn--small" title=ui.t("layers-delete") aria-label=ui.t("layers-delete")
                    prop:disabled=move || target().is_none()
                    on:click=move |_| act(target().map(|at| Command::Remove { at }))>{icons::trash()}</button>
            </footer>
            {move || menu.get().map(|(x, y)| view! {
                <div class="context-menu pop" role="menu" style=format!("position:fixed;left:{x}px;top:{y}px")>
                    {menu_item("layers-parent", "", icons::parent, Box::new(move || { if let Some(p) = parent_row() { select_row(&p) } }))}
                    {menu_item("layers-up", "", icons::arrow_up, Box::new(move || act(selected_row().and_then(|r| all.with_untracked(|a| step_command(a, &r, -1))))))}
                    {menu_item("layers-down", "", icons::arrow_down, Box::new(move || act(selected_row().and_then(|r| all.with_untracked(|a| step_command(a, &r, 1))))))}
                    <hr class="menu-sep"/>
                    {menu_item("layers-wrap", "", icons::wrap, Box::new(move || act(target().map(|at| Command::Wrap { at, tag: "div".into() }))))}
                    {menu_item("layers-duplicate", "key-duplicate", icons::copy, Box::new(move || act(target().map(|at| Command::Duplicate { at }))))}
                    <hr class="menu-sep"/>
                    {menu_item("layers-delete", "key-delete", icons::trash, Box::new(move || act(target().map(|at| Command::Remove { at }))))}
                </div>
            })}
        </aside>
    }
}
