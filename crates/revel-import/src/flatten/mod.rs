//! Styles mis à plat (D-157) : ce que le panneau Styles ne sait pas régler, ramené à des réglages
//! de classes, sans changer le rendu.
//!
//! Les pages sont rendues pour l'éditeur (repères `data-rv`) ; chaque règle est cherchée dans ces
//! pages (sélecteurs complexes compris, `selectors` de Servo par `dom_query`). Une règle que le
//! panneau ne sait pas régler (`.hero .lead`, `h1, h2`, `::after`, variables locales, attributs
//! de Framer…) est retirée et ses valeurs reportées sur une classe des éléments qu'elle touchait :
//! la classe de l'élément si elle ne sert qu'à eux, sinon une classe combinée ajoutée
//! (`.lead.lead--page-hero`). Le report est calculé par la cascade (spécificité, ordre,
//! `!important`, points de rupture, états `:hover`…, variables résolues pour chaque élément) :
//! avant et après, chaque élément reçoit les mêmes valeurs. Un pseudo-élément avec un contenu
//! devient un élément (`<span class="faq__question__after">+</span>`).
//!
//! Gardé tel quel (relevé) : règle qui touche un élément sans repère (texte riche du CMS, élément
//! ajouté par un script), qui touche un élément selon l'endroit où il est répété, état dans un
//! ancêtre hors variante de composant, requête média autre qu'une largeur.

pub mod media;
pub mod selector;
pub mod values;

use dom_query::{Document, Matcher, NodeId, NodeRef};
use revel_edit::styleaudit;
use revel_edit::{Branch, Command, Owner, Parent, Target};
use revel_model::{Attr, Decl, Node, Site, StyleItem, StyleRule};
use selector::Shape;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

/// Bilan de la mise à plat.
pub use revel_edit::styleaudit::FlattenReport as Report;

/// Propriétés retirées : doublons préfixés gérés par les navigateurs actuels, réglages du
/// navigateur sans effet sur la mise en page.
/// Balises de formulaire et d'interface (jamais écrites par un texte riche) : une règle qui en
/// demande une absente du site ne sert à rien.
const FORM_TAGS: &[&str] = &[
    "textarea", "select", "input", "option", "label", "form", "fieldset", "legend", "button",
    "dialog", "canvas", "progress", "meter", "output",
];

const DROPPED: &[&str] = &[
    "-webkit-text-size-adjust",
    "-moz-text-size-adjust",
    "text-size-adjust",
    "-webkit-font-smoothing",
    "-moz-osx-font-smoothing",
    "-webkit-tap-highlight-color",
    "will-change",
    "font-synthesis",
    "-webkit-font-feature-settings",
];

/// Niveau de la cascade : largeur d'écran et requête des règles écrites pour ce niveau.
#[derive(Debug, Clone)]
struct Level {
    width: f64,
    media: Option<String>,
}

/// Canal : état dans un ancêtre (`[open]`), état du sujet, pseudo-élément.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Channel {
    /// État d'un ancêtre (`[open]`, `.is-open`…).
    outer: Option<String>,
    /// Classes posées par un script sur l'élément (`.is-active`).
    extra: Vec<String>,
    state: Option<String>,
    pseudo: Option<String>,
}

/// Règle de travail : un sélecteur complexe.
struct WRule {
    item: usize,
    sel: String,
    media: Option<String>,
    /// Déclarations détaillées (raccourcis développés) pour la cascade.
    decls: Vec<Decl>,
    spec: (u32, u32, u32),
    shape: Option<Shape>,
    /// Règle que le panneau ne sait pas régler (sélecteur ou variables).
    problem: bool,
    /// Règle `:root` (variables du site) : jamais mise à plat.
    root: bool,
    matches: Vec<Occ>,
    /// Raison de la garder telle quelle.
    keep: Option<String>,
    /// Styles écrits sur un élément (attribut `style`) : son identifiant.
    inline: Option<String>,
}

/// Élément rendu : page et indice dans la liste des éléments de la page.
type Occ = (usize, usize);

struct Elem {
    rv: Option<String>,
    parent: Option<usize>,
    /// Variables écrites dans l'attribut `style` (priorité la plus haute).
    inline_vars: Vec<(String, String)>,
}

struct PageDoc {
    elems: Vec<Elem>,
}

/// Ordre des styles écrits sur un élément : après toutes les règles de la feuille, avant les
/// reports.
const INLINE_ORDER: usize = usize::MAX / 2;

/// Variables d'un élément (partagées avec son parent quand il n'en écrit pas).
type Vars = Rc<HashMap<String, String>>;

/// Valeur gagnante : valeur, priorité (`!important`, spécificité, ordre).
type Win = (String, Rank);

/// Priorité d'une déclaration : `!important`, spécificité, ordre.
type Rank = (bool, (u32, u32, u32), usize);

/// Met à plat les styles du site. `pages` : pages rendues pour l'éditeur (`data-rv`).
/// Renvoie les commandes (une seule suite à appliquer) et le bilan.
pub fn flatten(site: &Site, pages: &[String], content: &[String]) -> (Vec<Command>, Report) {
    flatten_with(site, pages, &[], content)
}

/// Comme [`flatten`], avec les composants rendus seuls (`components`) : leurs éléments comptent
/// seulement s'ils ne sont rendus dans aucune page (contenu par défaut d'un emplacement), sans
/// les ancêtres d'une page.
pub fn flatten_with(
    site: &Site,
    pages: &[String],
    components: &[String],
    content: &[String],
) -> (Vec<Command>, Report) {
    let n_pages = pages.len();
    let all: Vec<String> = pages.iter().chain(components).cloned().collect();
    let pages = &all[..];
    let mut report = Report::default();
    let Some(sheet) = &site.stylesheet else {
        return (Vec::new(), report);
    };
    let mut taken = site.node_ids();
    // 1. Listes de sélecteurs coupées, déclarations traduites.
    let mut items: Vec<StyleItem> = Vec::new();
    for it in &unwrap_supports(&sheet.items, &mut taken) {
        match it {
            StyleItem::Rule(r) if r.selector.trim() != ":root" => {
                let decls = translate_decls(&r.decls, &mut report);
                let sels = selector::split_list(&r.selector);
                let foundation = styleaudit::is_foundation(r);
                if sels.len() > 1 && !foundation {
                    for (k, s) in sels.into_iter().enumerate() {
                        let id = if k == 0 {
                            r.id.clone()
                        } else {
                            let id = revel_model::ids::fresh(&taken);
                            taken.insert(id.clone());
                            Some(id)
                        };
                        items.push(StyleItem::Rule(StyleRule {
                            id,
                            selector: s,
                            media: r.media.clone(),
                            decls: decls.clone(),
                        }));
                    }
                } else {
                    items.push(StyleItem::Rule(StyleRule { decls, ..r.clone() }));
                }
            }
            // Réglage propre à Safari (`-webkit-named-image` ne sert qu'à le reconnaître), badge
            // de Framer, ou repli des navigateurs sans couleurs `display-p3` ou sans `overflow: clip`
            // (tous les navigateurs actuels les connaissent : `color()` depuis Chrome 111, Safari 15,
            // Firefox 113 ; `overflow: clip` depuis Chrome 90, Safari 16, Firefox 81) : retiré.
            StyleItem::Raw { css }
                if css.trim_start().starts_with("@supports")
                    && (css.contains("-webkit-named-image")
                        || css.contains("#__framer-badge")
                        || {
                            let c = css.trim_start().replace(' ', "");
                            c.starts_with("@supportsnot(color:color(display-p3")
                                || c.starts_with("@supportsnot(overflow:clip)")
                        }) =>
            {
                report.dropped += 1;
            }
            other => items.push(other.clone()),
        }
    }
    // Boîtes : `box-sizing` écrit sur `html`, `body` ou `*` (avec `inherit`, comme Framer) → la
    // fondation de Revel (`*, *::before, *::after { box-sizing: border-box }`).
    foundation_box_sizing(&mut items, &mut taken, &mut report);
    // Variables écrites seulement dans `:root` et qui ne sont ni des couleurs ni des tailles du
    // site (polices…) : remplacées par leur valeur partout.
    substitute_root_only(site, &mut items);
    // Survol d'un composant qui change ses éléments (`.carte:hover .carte__image`) : variante
    // « Survol » du composant (ou « Appui »).
    let mut commands_pre: Vec<Command> = Vec::new();
    let mut components = site.components.clone();
    for it in &items {
        let StyleItem::Rule(r) = it else { continue };
        let Some(c) = selector::parse(&r.selector) else {
            continue;
        };
        if c.parts.len() < 2 {
            continue;
        }
        let first = &c.parts[0].1;
        let (cls, kind) = match first.as_slice() {
            [cls, st] if st == ":hover" => (cls, revel_model::VariantKind::Hover),
            [cls, st] if st == ":active" => (cls, revel_model::VariantKind::Pressed),
            _ => continue,
        };
        let Some(cls) = cls.strip_prefix('.') else {
            continue;
        };
        for comp in components.iter_mut() {
            if comp.root_class().as_deref() == Some(cls)
                && !comp.variants.iter().any(|v| v.kind == kind)
            {
                comp.variants.push(revel_model::Variant {
                    id: if kind == revel_model::VariantKind::Hover {
                        "survol".into()
                    } else {
                        "appui".into()
                    },
                    name: None,
                    kind,
                });
                commands_pre.push(Command::SetComponentVariants {
                    id: comp.id.clone(),
                    variants: comp.variants.clone(),
                });
            }
        }
    }
    // Points de rupture : ceux du site ; un site sans point de rupture (import) prend ceux de
    // ses requêtes média (largeurs maximales, et largeur minimale moins un).
    let mut bps = if site.breakpoints.is_empty() {
        revel_model::Breakpoint::defaults()
    } else {
        site.breakpoints.clone()
    };
    if bps
        .iter()
        .all(|b| b.max_width.is_none() && b.min_width.is_none())
    {
        let inferred = infer_breakpoints(&items);
        if inferred.len() > 1 {
            bps = inferred;
            commands_pre.insert(
                0,
                Command::SetBreakpoints {
                    breakpoints: bps.clone(),
                },
            );
        }
    }
    // 2. Site : variables, racines de variantes, nœuds et classes.
    let mut probe = site.clone();
    probe.components = components;
    if let Some(s) = probe.stylesheet.as_mut() {
        s.items = items.clone();
    }
    let site_vars = styleaudit::site_variables(&probe);
    let roots = styleaudit::variant_roots(&probe);
    let root_values: HashMap<String, String> = items
        .iter()
        .filter_map(|i| match i {
            StyleItem::Rule(r) if r.selector.trim() == ":root" && r.media.is_none() => Some(r),
            _ => None,
        })
        .flat_map(|r| r.decls.iter())
        .filter_map(|d| {
            d.prop
                .strip_prefix("--")
                .map(|n| (n.to_string(), d.value.trim().to_string()))
        })
        .collect();
    let nodes = node_index(site);
    let site_classes: HashSet<String> = nodes
        .values()
        .flat_map(|n| n.classes.iter().cloned())
        .collect();
    let site_tags: HashSet<String> = nodes.values().map(|n| n.tag.to_ascii_lowercase()).collect();
    let all_class_text: String = nodes
        .values()
        .map(|n| n.class_text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    // Variables lues dans les attributs des éléments (`style="width: var(--x)"`).
    let mut markup_refs: HashSet<String> = HashSet::new();
    for (_, _, body) in site.owners() {
        revel_model::ids::walk(body, &mut |n| {
            if let Node::Element { attrs, .. } | Node::Use { attrs, .. } = n {
                for a in attrs {
                    if let Some(v) = &a.value {
                        markup_refs.extend(styleaudit::var_refs(v).into_iter().map(str::to_string));
                    }
                }
            }
        });
    }
    // Variables lues par un bloc écrit en code (`@keyframes` : `translateX(var(--gap))`) : leur
    // valeur dépend de l'élément animé, leurs déclarations restent.
    for it in &items {
        if let StyleItem::Raw { css } = it {
            markup_refs.extend(styleaudit::var_refs(css).into_iter().map(str::to_string));
        }
    }
    // 3. Pages.
    let docs: Vec<Document> = pages.iter().map(|h| Document::from(h.as_str())).collect();
    // Textes riches des contenus : une règle qui n'y touche rien et rien dans les pages est morte.
    let content_docs: Vec<Document> = content.iter().map(|h| Document::from(h.as_str())).collect();
    let mut pdocs: Vec<PageDoc> = Vec::new();
    let mut idx_of: Vec<HashMap<NodeId, usize>> = Vec::new();
    for d in &docs {
        let mut elems = Vec::new();
        let mut map = HashMap::new();
        collect(d.root(), None, &mut elems, &mut map);
        pdocs.push(PageDoc { elems });
        idx_of.push(map);
    }
    // Classes des éléments rendus (textes riches des contenus compris).
    let page_classes: HashSet<String> = docs
        .iter()
        .flat_map(|d| {
            d.select("[class]")
                .nodes()
                .iter()
                .filter_map(|n| n.attr("class").map(|c| c.to_string()))
                .collect::<Vec<_>>()
        })
        .flat_map(|c| c.split_whitespace().map(str::to_string).collect::<Vec<_>>())
        .collect();
    let mut occs_of: HashMap<String, Vec<Occ>> = HashMap::new();
    for (p, pd) in pdocs.iter().enumerate() {
        let from_pages: HashSet<String> = if p >= n_pages {
            occs_of
                .iter()
                .filter(|(_, o)| o.iter().any(|&(q, _)| q < n_pages))
                .map(|(k, _)| k.clone())
                .collect()
        } else {
            HashSet::new()
        };
        for (e, el) in pd.elems.iter().enumerate() {
            if let Some(rv) = &el.rv
                && !from_pages.contains(rv)
            {
                occs_of.entry(rv.clone()).or_default().push((p, e));
            }
        }
    }
    // 4. Règles de travail.
    let bp_medias: Vec<String> = bps
        .iter()
        .filter_map(revel_model::Breakpoint::media)
        .collect();
    let mut rules: Vec<WRule> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        let StyleItem::Rule(r) = it else { continue };
        let root = r.selector.trim() == ":root";
        let complex = selector::parse(&r.selector);
        // Classes d'état (`is-…`) : posées ou retirées par les scripts du site, jamais figées.
        let dynamic = |c: &str| c.starts_with("is-");
        let shape = complex.as_ref().map(|c| selector::shape(c, &dynamic));
        let spec = complex
            .as_ref()
            .map(selector::specificity)
            .unwrap_or((0, 0, 0));
        let ok_sel =
            root || styleaudit::is_foundation(r) || styleaudit::panel_selector(&r.selector, &roots);
        let bad_var = !root
            && r.decls.iter().any(|d| {
                d.prop.starts_with("--") && !styleaudit::REVEL_PROPS.contains(&d.prop.as_str())
                    || styleaudit::var_refs(&d.value)
                        .iter()
                        .any(|v| !site_vars.contains(*v))
            });
        // Requête qui n'est pas un point de rupture du site : reportée aux points de rupture.
        let odd_media = !root && r.media.as_ref().is_some_and(|m| !bp_medias.contains(m));
        // Propriétés réécrites par élément (`flex-basis` seul → `flex` complet).
        let regroup = !root
            && r.decls.iter().any(|d| {
                values::GROUPS
                    .iter()
                    .any(|(_, parts)| parts.contains(&d.prop.as_str()))
            });
        let decls: Vec<Decl> = r
            .decls
            .iter()
            .flat_map(|d| {
                values::longhands(&d.prop, &d.value)
                    .into_iter()
                    .map(|(prop, value)| Decl {
                        prop,
                        value,
                        important: d.important,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        rules.push(WRule {
            item: i,
            sel: r.selector.clone(),
            media: r.media.clone(),
            decls,
            spec,
            shape,
            problem: !ok_sel || bad_var || regroup || odd_media,
            root,
            matches: Vec::new(),
            keep: None,
            inline: None,
        });
    }
    // Styles écrits sur les éléments (`style="…"`, sans expression) : règle propre à l'élément,
    // qui passe devant toutes les autres ; reportée sur une classe comme les autres.
    let mut inline_ids: Vec<&String> = nodes.keys().collect();
    inline_ids.sort();
    for rv in inline_ids {
        let n = &nodes[rv];
        let Some(style) = &n.style else { continue };
        if style.contains("{{") || style.contains("{%") || !occs_of.contains_key(rv) {
            continue;
        }
        let raw: Vec<Decl> = style
            .split(';')
            .filter_map(|d| {
                let (k, v) = d.split_once(':')?;
                let (v, important) = match v.trim().strip_suffix("!important") {
                    Some(x) => (x.trim().to_string(), true),
                    None => (v.trim().to_string(), false),
                };
                (!k.trim().is_empty() && !v.is_empty()).then(|| Decl {
                    prop: k.trim().to_ascii_lowercase(),
                    value: v,
                    important,
                })
            })
            .collect();
        let decls: Vec<Decl> = translate_decls(&raw, &mut report)
            .iter()
            .flat_map(|d| {
                values::longhands(&d.prop, &d.value)
                    .into_iter()
                    .map(|(prop, value)| Decl {
                        prop,
                        value,
                        important: d.important,
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        if decls.is_empty() {
            continue;
        }
        rules.push(WRule {
            item: INLINE_ORDER,
            sel: format!("[style] /* {rv} */"),
            media: None,
            decls,
            spec: (u32::MAX / 2, 0, 0),
            shape: Some(Shape {
                static_sel: String::new(),
                state: None,
                pseudo: None,
                context: None,
                unsupported: false,
                outer: None,
                extra: Vec::new(),
            }),
            problem: true,
            root: false,
            matches: occs_of[rv].clone(),
            keep: None,
            inline: Some(rv.clone()),
        });
    }
    // 5. Éléments touchés par chaque règle.
    let mut occ_rules: Vec<Vec<Vec<usize>>> = pdocs
        .iter()
        .map(|p| vec![Vec::new(); p.elems.len()])
        .collect();
    for (ri, r) in rules.iter_mut().enumerate() {
        if r.inline.is_some() {
            for &(p, e) in &r.matches {
                occ_rules[p][e].push(ri);
            }
            continue;
        }
        let sel = match &r.shape {
            Some(s) => s.static_sel.clone(),
            None => r.sel.clone(),
        };
        // `:root` : l'élément `html` (non reconnu par le moteur de sélecteurs).
        let sel = if r.root {
            "html".to_string()
        } else {
            sel.replace(":root", "html")
        };
        let Ok(m) = Matcher::new(&sel) else {
            r.keep = Some("sélecteur illisible".into());
            continue;
        };
        for (p, d) in docs.iter().enumerate() {
            for n in d.select_matcher(&m).nodes() {
                if let Some(&e) = idx_of[p].get(&n.id) {
                    // Composant seul : seulement ses éléments que les pages ne rendent pas.
                    if p >= n_pages
                        && pdocs[p].elems[e]
                            .rv
                            .as_ref()
                            .is_some_and(|rv| !occs_of[rv].contains(&(p, e)))
                    {
                        continue;
                    }
                    r.matches.push((p, e));
                    occ_rules[p][e].push(ri);
                }
            }
        }
    }
    // 6. Niveaux (largeurs) de la cascade.
    let levels = levels(&bps);
    let applies = |r: &WRule, l: &Level| -> bool {
        match &r.media {
            None => true,
            Some(m) => media::eval(m, l.width) == media::Eval::Yes,
        }
    };
    // 7. Règles à mettre à plat.
    let mut dead: HashSet<usize> = HashSet::new();
    let mut simplify: Vec<usize> = Vec::new();
    for (ri, r) in rules.iter_mut().enumerate() {
        if !r.problem || r.root || r.keep.is_some() {
            continue;
        }
        let Some(shape) = &r.shape else {
            r.keep = Some("sélecteur non reconnu".into());
            continue;
        };
        if r.matches.is_empty()
            && dead_rule(r, &site_classes, &page_classes, &all_class_text, &site_tags)
        {
            dead.insert(ri);
            continue;
        }
        if shape.unsupported {
            r.keep = Some("état dans un ancêtre ou pseudo-élément non reporté".into());
            continue;
        }
        // `hidden` : attribut posé ou retiré par un script (bandeau de consentement, menu) ; le
        // report figerait l'élément dans l'état de la page rendue.
        if r.sel.contains("[hidden") {
            r.keep = Some("attribut hidden, posé par un script".into());
            continue;
        }
        if let Some(m) = &r.media
            && levels
                .iter()
                .any(|l| media::eval(m, l.width) == media::Eval::Unknown)
        {
            r.keep = Some(format!("requête @media {m}"));
            continue;
        }
        if let Some(p) = &shape.pseudo
            && !matches!(p.as_str(), "before" | "after" | "-webkit-details-marker")
        {
            r.keep = Some(format!("pseudo-élément ::{p}"));
            continue;
        }
        if r.decls.iter().any(|d| {
            d.prop
                .strip_prefix("--")
                .is_some_and(|n| markup_refs.contains(n))
        }) {
            r.keep = Some("variable lue dans un attribut ou une animation".into());
            continue;
        }
        // Triangle des accordéons de Safari : `list-style-type: none` du résumé le retire
        // (Safari 18.4 et suivants le traitent comme une puce).
        if shape.pseudo.as_deref() == Some("-webkit-details-marker") {
            dead.insert(ri);
            continue;
        }
        if r.matches.is_empty() {
            // Classe ou balise de formulaire absente du site : règle morte.
            let missing = dead_rule(r, &site_classes, &page_classes, &all_class_text, &site_tags);
            // Ni élément du site qui puisse le porter (balise, classes, attributs du sujet), ni
            // texte riche des contenus qu'il touche : règle morte.
            let in_content = Matcher::new(&shape.static_sel).ok().is_some_and(|m| {
                content_docs
                    .iter()
                    .any(|d| !d.select_matcher(&m).is_empty())
            });
            // Chaque partie du sélecteur doit pouvoir être portée par un élément du site.
            let in_model = selector::parse(&shape.static_sel).is_some_and(|c| {
                c.parts
                    .iter()
                    .all(|(_, part)| nodes.values().any(|n| subject_fits(part, n)))
            });
            if missing || (!in_content && !in_model) {
                dead.insert(ri);
            } else {
                r.keep = Some("texte riche des contenus".into());
            }
            continue;
        }
        // Élément du site qui peut porter la règle sans avoir été rendu (branche d'une condition,
        // page non échantillonnée) : il garderait sa classe sans le report, la règle reste.
        // Ancêtres du sélecteur cherchés parmi les éléments du même propriétaire (page, gabarit,
        // composant).
        if let Some(c) = selector::parse(&shape.static_sel)
            && let Some((_, subject)) = c.parts.last()
            && nodes.iter().any(|(id, n)| {
                n.element
                    && !occs_of.contains_key(id)
                    && subject_fits(subject, n)
                    && c.parts[..c.parts.len() - 1].iter().all(|(_, part)| {
                        nodes
                            .values()
                            .any(|m| m.owner == n.owner && subject_fits(part, m))
                    })
            })
        {
            r.keep = Some("élément non rendu".into());
            continue;
        }
        if r.matches
            .iter()
            .any(|&(p, e)| pdocs[p].elems[e].rv.is_none())
        {
            r.keep = Some("élément sans repère (texte riche, élément ajouté par un script)".into());
            simplify.push(ri);
            continue;
        }
        let _ = ri;
    }
    // Balise d'un texte riche dans une classe, sous d'autres ancêtres (`.docs .prose h2`) : quand
    // chaque `.prose` est sous ces ancêtres, `.prose h2` touche les mêmes éléments.
    for ri in simplify {
        let Some(c) = selector::parse(&rules[ri].sel) else {
            continue;
        };
        let n = c.parts.len();
        if n < 3
            || c.parts
                .iter()
                .skip(1)
                .any(|(comb, _)| *comb != selector::Comb::Desc)
            || rules[ri]
                .media
                .as_ref()
                .is_some_and(|m| !bp_medias.contains(m))
        {
            continue;
        }
        let (holder, tag) = (&c.parts[n - 2].1, &c.parts[n - 1].1);
        let [h] = holder.as_slice() else { continue };
        let [t] = tag.as_slice() else { continue };
        if !h.starts_with('.') || !styleaudit::NESTED_TAGS.contains(&t.as_str()) {
            continue;
        }
        let anc = selector::Complex {
            parts: c.parts[..n - 1].to_vec(),
        }
        .write();
        let (Ok(all), Ok(inside)) = (Matcher::new(h), Matcher::new(&anc)) else {
            continue;
        };
        let same = docs
            .iter()
            .all(|d| d.select_matcher(&all).length() == d.select_matcher(&inside).length());
        let new_sel = format!("{h} {t}");
        let clash = rules.iter().any(|o| o.sel == new_sel);
        if same && !clash {
            if let StyleItem::Rule(r) = &mut items[rules[ri].item] {
                r.selector = new_sel.clone();
            }
            rules[ri].sel = new_sel;
            rules[ri].keep = None;
            rules[ri].problem = rules[ri].decls.iter().any(|d| {
                d.prop.starts_with("--")
                    || styleaudit::var_refs(&d.value)
                        .iter()
                        .any(|v| !site_vars.contains(*v))
            });
            if rules[ri].problem {
                rules[ri].keep = Some("variable dans un texte riche".into());
            }
        }
    }
    // Cohérence : chaque élément touché l'est partout où il est rendu.
    loop {
        let mut changed = false;
        for rule in rules.iter_mut() {
            if !flat_candidate(rule) {
                continue;
            }
            let matched: HashSet<Occ> = rule.matches.iter().copied().collect();
            let mut bad = None;
            for &(p, e) in &rule.matches {
                let rv = pdocs[p].elems[e].rv.clone().unwrap_or_default();
                if occs_of[&rv].iter().any(|o| !matched.contains(o)) {
                    bad = Some("élément touché à un endroit seulement".to_string());
                    break;
                }
                if rule
                    .shape
                    .as_ref()
                    .and_then(|s| s.pseudo.as_ref())
                    .is_some()
                    && !nodes.get(&rv).is_some_and(|n| n.element)
                {
                    bad = Some("pseudo-élément d'un élément qui n'en reçoit pas".to_string());
                    break;
                }
            }
            if let Some(b) = bad {
                rule.keep = Some(b);
                changed = true;
            }
        }
        // Pseudo-élément aussi touché par une règle gardée : gardé en entier.
        let mut kept_pseudo: HashSet<(String, String)> = HashSet::new();
        for r in &rules {
            if r.keep.is_some()
                && let Some(p) = r.shape.as_ref().and_then(|s| s.pseudo.clone())
            {
                for &(pg, e) in &r.matches {
                    if let Some(rv) = &pdocs[pg].elems[e].rv {
                        kept_pseudo.insert((rv.clone(), p.clone()));
                    }
                }
            }
        }
        for r in rules.iter_mut() {
            if !flat_candidate(r) {
                continue;
            }
            if let Some(p) = r.shape.as_ref().and_then(|s| s.pseudo.clone())
                && r.matches.iter().any(|&(pg, e)| {
                    pdocs[pg].elems[e]
                        .rv
                        .as_ref()
                        .is_some_and(|rv| kept_pseudo.contains(&(rv.clone(), p.clone())))
                })
            {
                r.keep = Some("pseudo-élément aussi réglé par une règle gardée".into());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // 8. Variables de chaque élément, à chaque niveau.
    let site_var_kept = |name: &str, cp: &HashMap<String, String>| {
        site_vars.contains(name) && cp.get(name) == root_values.get(name)
    };
    let mut cps: Vec<Vec<Vec<Vars>>> = Vec::new(); // [niveau][page][élément]
    for l in &levels {
        let mut per_page = Vec::new();
        for (p, pd) in pdocs.iter().enumerate() {
            let mut v: Vec<Rc<HashMap<String, String>>> = Vec::with_capacity(pd.elems.len());
            for (e, el) in pd.elems.iter().enumerate() {
                let parent: Rc<HashMap<String, String>> = el
                    .parent
                    .map(|q| v[q].clone())
                    .unwrap_or_else(|| Rc::new(HashMap::new()));
                let mut own: Vec<(String, String)> = winners(
                    &rules,
                    &occ_rules[p][e],
                    l,
                    &Channel::default(),
                    &|_| true,
                    &applies,
                    true,
                )
                .into_iter()
                .map(|(k, (v, _))| (k.trim_start_matches("--").to_string(), v))
                .collect();
                for (k, v) in &el.inline_vars {
                    own.retain(|(n, _)| n != k);
                    own.push((k.clone(), v.clone()));
                }
                if own.is_empty() {
                    v.push(parent);
                    continue;
                }
                let mut env: HashMap<String, String> = (*parent).clone();
                for (k, val) in &own {
                    env.insert(k.clone(), val.clone());
                }
                let mut out = (*parent).clone();
                for (name, val) in &own {
                    let name = name.clone();
                    let keep = |n: &str| {
                        n != name && site_vars.contains(n) && env.get(n) == root_values.get(n)
                    };
                    match values::substitute(val, &env, &keep) {
                        Some(x) => {
                            out.insert(name, x);
                        }
                        None => {
                            out.remove(&name);
                        }
                    }
                }
                v.push(Rc::new(out));
            }
            per_page.push(v);
        }
        cps.push(per_page);
    }
    // 9. Valeurs avant, valeurs après sans les règles mises à plat, report par élément.
    // Itéré : un élément dont les valeurs diffèrent d'un endroit à l'autre garde ses règles.
    let mut plans: BTreeMap<String, ElementPlan>;
    loop {
        plans = BTreeMap::new();
        let flat: HashSet<usize> = (0..rules.len())
            .filter(|&i| flat_candidate(&rules[i]))
            .collect();
        let mut affected: BTreeSet<String> = BTreeSet::new();
        for &ri in &flat {
            for &(p, e) in &rules[ri].matches {
                if let Some(rv) = &pdocs[p].elems[e].rv {
                    affected.insert(rv.clone());
                }
            }
        }
        let mut inconsistent: HashSet<String> = HashSet::new();
        for rv in &affected {
            let occs = &occs_of[rv];
            let mut plan: Option<ElementPlan> = None;
            let ncls = nodes.get(rv).map_or(0, |n| n.classes.len() as u32);
            for &(p, e) in occs {
                let channels = channels_of(&rules, &occ_rules[p][e], &flat);
                let mut ep = ElementPlan::default();
                let mut lost = false;
                let mut t_so_far: Vec<(usize, Channel, BTreeMap<String, String>)> = Vec::new();
                for (li, l) in levels.iter().enumerate() {
                    let cp = &cps[li][p][e];
                    for ch in &channels {
                        let resolve = |w: BTreeMap<String, Win>| -> BTreeMap<String, String> {
                            w.into_iter()
                                .filter(|(k, _)| !k.starts_with("--"))
                                .map(|(k, (v, _))| {
                                    let v = resolve_value(&k, &v, cp, &site_var_kept);
                                    (k, v)
                                })
                                .collect()
                        };
                        let before = resolve(winners(
                            &rules,
                            &occ_rules[p][e],
                            l,
                            ch,
                            &|_| true,
                            &applies,
                            false,
                        ));
                        let mut after_w = winners(
                            &rules,
                            &occ_rules[p][e],
                            l,
                            ch,
                            &|ri| !flat.contains(&ri),
                            &applies,
                            false,
                        );
                        if ch.pseudo.is_some() {
                            // L'élément ajouté n'a que son report.
                            after_w.clear();
                        }
                        // Report déjà écrit aux niveaux plus larges (qui s'applique ici) : placé
                        // après toutes les règles, spécificité des classes de l'élément plus une.
                        let t_spec = (
                            0,
                            ncls + 1
                                + u32::from(ch.state.is_some())
                                + u32::from(ch.outer.is_some())
                                + ch.extra.len() as u32,
                            0,
                        );
                        for (tli, tch, decls) in &t_so_far {
                            if tch == ch && level_applies(&levels[*tli], l) {
                                for (k, v) in decls {
                                    let key = (false, t_spec, usize::MAX - levels.len() + tli);
                                    let wins = after_w.get(k).is_none_or(|(_, old)| *old < key);
                                    if wins {
                                        after_w.insert(k.clone(), (v.clone(), key));
                                    }
                                }
                            }
                        }
                        // Priorité des valeurs gardées : le report de ce niveau doit passer devant.
                        let after_keys: BTreeMap<String, Rank> = after_w
                            .iter()
                            .map(|(k, (_, key))| (k.clone(), *key))
                            .collect();
                        let t_key = (false, t_spec, usize::MAX - levels.len() + li);
                        // Valeurs du report déjà résolues : pas de nouvelle substitution.
                        let after: BTreeMap<String, String> = after_w
                            .into_iter()
                            .filter(|(k, _)| !k.starts_with("--"))
                            .map(|(k, (v, key))| {
                                if key.2 >= usize::MAX - levels.len() {
                                    (k, v)
                                } else {
                                    let v = resolve_value(&k, &v, cp, &site_var_kept);
                                    (k, v)
                                }
                            })
                            .collect();
                        let mut t = BTreeMap::new();
                        let props: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
                        for k in props {
                            if ch.pseudo.is_some() && k == "content" {
                                continue;
                            }
                            // `unset` n'est pas une absence : il passe devant le style du navigateur
                            // (lien bleu, `strong` en gras).
                            match (before.get(k), after.get(k)) {
                                (Some(b), Some(a)) if a == b => {}
                                (Some(b), _) => {
                                    t.insert(k.clone(), b.clone());
                                }
                                (None, Some(_)) => {
                                    // Mot-clé écrit avant (`unset`) gardé, sinon valeur du
                                    // navigateur.
                                    let v = before
                                        .get(k)
                                        .cloned()
                                        .unwrap_or_else(|| "revert".to_string());
                                    t.insert(k.clone(), v);
                                }
                                (None, None) => {}
                            }
                        }
                        // Contenu du pseudo-élément : celui de son canal de base (ni état ni
                        // accordéon ouvert), au niveau le plus large.
                        if let Some(pn) = &ch.pseudo
                            && li == 0
                            && ch.outer.is_none()
                            && ch.extra.is_empty()
                            && ch.state.is_none()
                        {
                            let content = before.get("content").cloned().unwrap_or_default();
                            ep.pseudo_content.insert(pn.clone(), content);
                        }
                        // Une règle gardée plus forte que le report (`!important`, identifiant) :
                        // l'élément garde ses règles.
                        if t.keys()
                            .any(|k| after_keys.get(k).is_some_and(|key| *key > t_key))
                        {
                            lost = true;
                        }
                        if !t.is_empty() {
                            // Groupes réglés par leur raccourci (`flex`, `overflow`) : écrits
                            // complets ; la cascade suivante voit les mêmes valeurs détaillées.
                            let written = values::recompose(&t, &before);
                            let mut seen = t.clone();
                            for (k, v) in &written {
                                for (lk, lv) in values::longhands(k, v) {
                                    seen.insert(lk, lv);
                                }
                            }
                            ep.t.insert((li, ch.clone()), written);
                            t_so_far.push((li, ch.clone(), seen));
                        }
                    }
                }
                ep.context = context_of(&rules, &occ_rules[p][e], &flat);
                if lost {
                    inconsistent.insert(rv.clone());
                    break;
                }
                match &plan {
                    None => plan = Some(ep),
                    Some(prev) if prev.t != ep.t || prev.pseudo_content != ep.pseudo_content => {
                        inconsistent.insert(rv.clone());
                        break;
                    }
                    _ => {}
                }
            }
            if let Some(pl) = plan {
                plans.insert(rv.clone(), pl);
            }
        }
        // Règle gardée qui touche un élément sous une condition que les niveaux ne calculent pas
        // (état posé par un script, `.x:hover`, mouvement réduit…) : le report de base, écrit
        // après elle avec une spécificité au moins égale, l'emporterait. Une règle d'état simple
        // est reportée elle aussi (après la base, sur la même classe) ; sinon, les règles de
        // l'élément restent telles quelles.
        let mut promote: BTreeSet<usize> = BTreeSet::new();
        let mut blocked: HashSet<String> = HashSet::new();
        for (rv, pl) in &plans {
            let base_props: HashSet<String> =
                pl.t.iter()
                    .filter(|((_, c), _)| *c == Channel::default())
                    .flat_map(|(_, d)| {
                        d.iter()
                            .flat_map(|(k, v)| values::longhands(k, v).into_iter().map(|(p, _)| p))
                            .collect::<Vec<_>>()
                    })
                    .collect();
            if base_props.is_empty() {
                continue;
            }
            let ncls = nodes.get(rv).map_or(0, |n| {
                n.classes.iter().filter(|c| !c.starts_with("is-")).count() as u32
            });
            for &(p, e) in &occs_of[rv] {
                for &ri in &occ_rules[p][e] {
                    let r = &rules[ri];
                    if r.root || flat.contains(&ri) {
                        continue;
                    }
                    let unknown_media = r.media.as_ref().is_some_and(|m| {
                        levels
                            .iter()
                            .any(|l| media::eval(m, l.width) == media::Eval::Unknown)
                    });
                    let ch = r.shape.as_ref().map(channel_of).unwrap_or_default();
                    if (ch == Channel::default() && !unknown_media)
                        || r.spec > (0, ncls + 1, 0)
                        || !r.decls.iter().any(|d| base_props.contains(&d.prop))
                    {
                        continue;
                    }
                    let promotable = !r.problem
                        && r.keep.is_none()
                        && !unknown_media
                        && ch.pseudo.is_none()
                        && r.shape.as_ref().is_some_and(|s| !s.unsupported);
                    if promotable {
                        promote.insert(ri);
                    } else {
                        blocked.insert(rv.clone());
                    }
                }
            }
        }
        if !promote.is_empty() || !blocked.is_empty() {
            for ri in promote {
                rules[ri].problem = true;
            }
            for r in rules.iter_mut() {
                if flat_candidate(r)
                    && r.matches.iter().any(|&(p, e)| {
                        pdocs[p].elems[e]
                            .rv
                            .as_ref()
                            .is_some_and(|rv| blocked.contains(rv))
                    })
                {
                    r.keep = Some("règle sous condition (état, mouvement réduit) gardée".into());
                }
            }
            continue;
        }
        if inconsistent.is_empty() {
            break;
        }
        for r in rules.iter_mut() {
            if flat_candidate(r)
                && r.matches.iter().any(|&(p, e)| {
                    pdocs[p].elems[e]
                        .rv
                        .as_ref()
                        .is_some_and(|rv| inconsistent.contains(rv))
                })
            {
                r.keep = Some("valeurs différentes selon l'endroit où l'élément est rendu".into());
            }
        }
    }
    // Pseudo-élément sans contenu : pas d'élément (le pseudo-élément ne s'affichait pas).
    for pl in plans.values_mut() {
        pl.pseudo_content
            .retain(|_, c| !matches!(c.trim(), "" | "none" | "normal"));
        // Report sur un pseudo-élément sans contenu : retiré (il ne s'affichait pas).
        let with_content: HashSet<String> = pl.pseudo_content.keys().cloned().collect();
        pl.t.retain(|(_, c), _| c.pseudo.as_ref().is_none_or(|p| with_content.contains(p)));
    }
    // 10. Classes des reports : classe de l'élément si elle ne sert qu'aux éléments qui reçoivent
    // le même report, sinon classe combinée ajoutée.
    let mut commands: Vec<Command> = commands_pre;
    let mut new_rules: Vec<(usize, Channel, String, BTreeMap<String, String>)> = Vec::new(); // (level, channel, selector, decls)
    let mut used_names: HashSet<String> = site_classes.clone();
    let mut class_adds: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // Regroupement par classes de l'élément et report identique (hors pseudo-éléments).
    let mut groups: BTreeMap<(Vec<String>, String), Vec<String>> = BTreeMap::new();
    for (rv, pl) in &plans {
        let base: BTreeMap<_, _> =
            pl.t.iter()
                .filter(|((_, c), _)| c.pseudo.is_none())
                .collect();
        if base.is_empty() {
            continue;
        }
        // Classes d'état (`is-active`), posées et retirées par un script : jamais cible d'un report.
        let classes: Vec<String> = nodes
            .get(rv)
            .map(|n| {
                n.classes
                    .iter()
                    .filter(|c| !c.starts_with("is-"))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let key = format!("{base:?}");
        groups.entry((classes, key)).or_default().push(rv.clone());
    }
    for ((classes, _), rvs) in &groups {
        let pl = &plans[&rvs[0]];
        let group: HashSet<&String> = rvs.iter().collect();
        // Tous les éléments du site qui ont ces classes sont dans le groupe : classe gardée.
        // Classe de l'élément gardée seule : sa spécificité (une classe de moins que la classe
        // combinée) doit encore l'emporter sur les règles gardées qui touchent l'élément.
        let flat_now: HashSet<usize> = (0..rules.len())
            .filter(|&i| flat_candidate(&rules[i]))
            .collect();
        let fold_wins = || {
            rvs.iter().all(|rv| {
                occs_of[rv].iter().all(|&(p, e)| {
                    pl.t.iter().all(|((li, ch), decls)| {
                        let spec = (
                            0,
                            classes.len() as u32
                                + u32::from(ch.state.is_some())
                                + u32::from(ch.outer.is_some())
                                + ch.extra.len() as u32,
                            0,
                        );
                        let props: HashSet<String> = decls
                            .iter()
                            .flat_map(|(k, v)| values::longhands(k, v).into_iter().map(|(p, _)| p))
                            .collect();
                        occ_rules[p][e].iter().all(|&ri| {
                            let r = &rules[ri];
                            flat_now.contains(&ri)
                                || !applies(r, &levels[*li])
                                || r.shape.as_ref().map(channel_of).unwrap_or_default() != *ch
                                || !r.decls.iter().any(|d| {
                                    props.contains(&d.prop) && (d.important || r.spec > spec)
                                })
                        })
                    })
                })
            })
        };
        let fold = !classes.is_empty()
            && nodes
                .iter()
                .filter(|(_, n)| classes.iter().all(|c| n.classes.contains(c)))
                .all(|(id, _)| group.contains(id))
            && fold_wins();
        let sel = if fold {
            classes.iter().map(|c| format!(".{c}")).collect::<String>()
        } else {
            let base = classes
                .first()
                .cloned()
                .or_else(|| nodes.get(&rvs[0]).map(|n| n.tag.clone()))
                .unwrap_or_else(|| "element".into());
            let name = unique_name(
                &format!(
                    "{base}--{}",
                    pl.context.clone().unwrap_or_else(|| "variante".into())
                ),
                &mut used_names,
            );
            for rv in rvs {
                class_adds.entry(rv.clone()).or_default().push(name.clone());
            }
            report.classes += 1;
            let mut s: String = classes.iter().map(|c| format!(".{c}")).collect();
            s.push_str(&format!(".{name}"));
            s
        };
        for ((li, ch), decls) in &pl.t {
            if ch.pseudo.is_some() {
                continue;
            }
            new_rules.push((*li, ch.clone(), sel.clone(), decls.clone()));
        }
    }
    // Pseudo-éléments : un élément ajouté, sa classe.
    let mut pseudo_groups: BTreeMap<(String, String, String), Vec<(String, String)>> =
        BTreeMap::new();
    for (rv, pl) in &plans {
        for (p, content) in &pl.pseudo_content {
            let p = p.clone();
            let decls: BTreeMap<_, _> =
                pl.t.iter()
                    .filter(|((_, c), _)| c.pseudo.as_deref() == Some(p.as_str()))
                    .collect();
            let base = nodes
                .get(rv)
                .and_then(|n| n.classes.first().cloned())
                .unwrap_or_else(|| nodes.get(rv).map(|n| n.tag.clone()).unwrap_or_default());
            pseudo_groups
                .entry((base, p, format!("{decls:?}{content}")))
                .or_default()
                .push((rv.clone(), content.clone()));
        }
    }
    for ((base, p, _), members) in &pseudo_groups {
        let name = unique_name(&format!("{base}__{p}"), &mut used_names);
        let (rv0, _) = &members[0];
        let pl = &plans[rv0];
        for ((li, ch), decls) in &pl.t {
            if ch.pseudo.as_deref() == Some(p.as_str()) {
                new_rules.push((
                    *li,
                    Channel {
                        pseudo: None,
                        ..ch.clone()
                    },
                    format!(".{name}"),
                    decls.clone(),
                ));
            }
        }
        for (rv, content) in members {
            let Some(n) = nodes.get(rv) else { continue };
            let text = css_string(content);
            let node = Node::Element {
                id: Some({
                    let id = revel_model::ids::fresh(&taken);
                    taken.insert(id.clone());
                    id
                }),
                tag: "span".into(),
                attrs: vec![
                    Attr {
                        name: "class".into(),
                        value: Some(name.clone()),
                        when: None,
                    },
                    Attr {
                        name: "aria-hidden".into(),
                        value: Some("true".into()),
                        when: None,
                    },
                ],
                children: if text.is_empty() {
                    Vec::new()
                } else {
                    vec![Node::Text { text }]
                },
            };
            let target = Target {
                owner: n.owner.clone(),
                node: rv.clone(),
            };
            let index = if p == "before" { 0 } else { n.children };
            commands.push(Command::Insert {
                parent: Parent::of(&target, Branch::Children),
                index,
                node,
            });
            report.pseudo += 1;
        }
    }
    // Classes ajoutées aux éléments.
    for (rv, add) in &class_adds {
        let Some(n) = nodes.get(rv) else { continue };
        let target = Target {
            owner: n.owner.clone(),
            node: rv.clone(),
        };
        let mut v = n.class_text.trim().to_string();
        for a in add {
            if !v.is_empty() {
                v.push(' ');
            }
            v.push_str(a);
        }
        if n.image {
            if let Some(mut img) = n.image_node.clone() {
                img.class = Some(v);
                commands.push(Command::SetImage {
                    at: target,
                    image: img,
                });
            }
        } else {
            commands.push(Command::SetAttr {
                at: target,
                name: "class".into(),
                value: Some(v),
            });
        }
    }
    // 11. Feuille finale : règles mises à plat retirées, reports ajoutés après (du plus large au
    // plus étroit), variables qui ne servent plus retirées.
    let flat: HashSet<usize> = (0..rules.len())
        .filter(|&i| flat_candidate(&rules[i]))
        .collect();
    let removed_items: HashSet<usize> = flat
        .iter()
        .chain(dead.iter())
        .filter(|&&ri| rules[ri].inline.is_none())
        .map(|&ri| rules[ri].item)
        .collect();
    // Styles écrits sur les éléments reportés : attribut retiré, sauf les variables qu'une règle
    // gardée lit encore.
    let still_read: HashSet<String> = rules
        .iter()
        .enumerate()
        .filter(|(i, _)| !flat.contains(i) && !dead.contains(i))
        .flat_map(|(_, r)| r.decls.iter())
        .flat_map(|d| {
            styleaudit::var_refs(&d.value)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect();
    for &ri in &flat {
        let Some(rv) = &rules[ri].inline else {
            continue;
        };
        let Some(n) = nodes.get(rv) else { continue };
        let target = Target {
            owner: n.owner.clone(),
            node: rv.clone(),
        };
        let keep: Vec<String> = n
            .style
            .as_deref()
            .unwrap_or_default()
            .split(';')
            .filter(|d| {
                d.split_once(':')
                    .and_then(|(k, _)| k.trim().strip_prefix("--"))
                    .is_some_and(|name| still_read.contains(name))
            })
            .map(|d| d.trim().to_string())
            .collect();
        commands.push(if keep.is_empty() {
            Command::RemoveAttr {
                at: target,
                name: "style".into(),
            }
        } else {
            Command::SetAttr {
                at: target,
                name: "style".into(),
                value: Some(keep.join(";")),
            }
        });
    }
    report.flattened = flat.len();
    report.dead = dead.len();
    let mut final_items: Vec<StyleItem> = items
        .iter()
        .enumerate()
        .filter(|(i, _)| !removed_items.contains(i))
        .map(|(_, it)| it.clone())
        .collect();
    // Valeur initiale d'une propriété sans réglage dans le panneau, que rien d'autre ne règle
    // (Framer remet ses variables de texte à zéro) : rien à écrire.
    let set_elsewhere: HashSet<String> = final_items
        .iter()
        .filter_map(|i| match i {
            StyleItem::Rule(r) => Some(r),
            _ => None,
        })
        .flat_map(|r| r.decls.iter())
        .filter(|d| !is_initial_value(&d.prop, &d.value))
        .map(|d| d.prop.clone())
        .chain(
            new_rules
                .iter()
                .flat_map(|(_, _, _, d)| d.iter())
                .filter(|(p, v)| !is_initial_value(p, v))
                .map(|(p, _)| p.clone()),
        )
        .collect();
    for (_, _, _, decls) in new_rules.iter_mut() {
        decls.retain(|p, v| {
            styleaudit::PANEL_PROPS.contains(&p.as_str())
                || set_elsewhere.contains(p)
                || !is_initial_value(p, v)
        });
    }
    new_rules.retain(|(_, _, _, d)| !d.is_empty());
    new_rules.sort_by_key(|(li, ch, _, _)| (*li, ch.clone()));
    for (li, ch, sel, decls) in new_rules {
        let mut s = String::new();
        if let Some(o) = &ch.outer {
            s.push_str(o);
            s.push(' ');
        }
        s.push_str(&sel);
        for x in &ch.extra {
            s.push_str(x);
        }
        if let Some(st) = &ch.state {
            s.push_str(st);
        }
        let id = revel_model::ids::fresh(&taken);
        taken.insert(id.clone());
        final_items.push(StyleItem::Rule(StyleRule {
            id: Some(id),
            selector: s,
            media: levels[li].media.clone(),
            decls: decls
                .into_iter()
                .map(|(prop, value)| Decl {
                    prop,
                    value,
                    important: false,
                })
                .collect(),
        }));
        report.written += 1;
    }
    clean_variables(&mut final_items, &site_vars, &markup_refs);
    final_items.retain(|i| !matches!(i, StyleItem::Rule(r) if r.decls.is_empty()));
    for (ri, r) in rules.iter().enumerate() {
        if r.problem && !r.root && !flat.contains(&ri) && !dead.contains(&ri) {
            let label = match &r.media {
                Some(m) => format!("@media {m} {{ {} }}", r.sel),
                None => r.sel.clone(),
            };
            report.kept.push((
                label,
                r.keep
                    .clone()
                    .unwrap_or_else(|| "variable ou réglage gardé".into()),
            ));
        }
    }
    // Chaque règle remise a un identifiant (règles écrites par un import sans identifiant).
    for it in final_items.iter_mut() {
        if let StyleItem::Rule(r) = it
            && r.id.is_none()
        {
            let id = revel_model::ids::fresh(&taken);
            taken.insert(id.clone());
            r.id = Some(id);
        }
    }
    // Feuille réécrite : tout retirer puis tout remettre (une seule suite annulable).
    if final_items != sheet.items {
        for i in (0..sheet.items.len()).rev() {
            commands.push(Command::RemoveStyleAt { index: i });
        }
        for (i, it) in final_items.into_iter().enumerate() {
            commands.push(Command::InsertStyle { index: i, item: it });
        }
    }
    (commands, report)
}

/// Valeur d'une déclaration pour un élément : variables remplacées (celles du site gardées),
/// propriété détaillée d'un raccourci à variable reprise de la valeur du raccourci ; `unset` si
/// une variable manque (déclaration sans effet, comme dans le navigateur).
fn resolve_value(
    prop: &str,
    v: &str,
    cp: &HashMap<String, String>,
    site_var_kept: &dyn Fn(&str, &HashMap<String, String>) -> bool,
) -> String {
    let keep = |n: &str| site_var_kept(n, cp);
    let sub = |raw: &str| values::substitute(raw, cp, &keep);
    if v.starts_with(values::PENDING) {
        let all = |raw: &str| values::substitute(raw, cp, &|_: &str| false);
        return values::pending(prop, v, &sub, &all).unwrap_or_else(|| "unset".to_string());
    }
    sub(v).unwrap_or_else(|| "unset".to_string())
}

/// Un élément du site peut porter le sujet du sélecteur (balise, classes, attributs ; les
/// pseudo-classes ne sont pas vérifiées).
fn subject_fits(subject: &[String], n: &NodeInfo) -> bool {
    subject.iter().all(|t| {
        if let Some(c) = t.strip_prefix('.') {
            n.classes.iter().any(|x| x == c) || n.class_text.contains(c)
        } else if let Some(a) = t.strip_prefix('[') {
            let name: String = a
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            n.attrs.contains(&name.to_ascii_lowercase())
        } else if t.starts_with('#') {
            n.attrs.contains("id")
        } else if t.starts_with(':') || t == "*" {
            true
        } else {
            n.tag.eq_ignore_ascii_case(t)
        }
    })
}

/// Valeur initiale (ou celle que le navigateur donne déjà) d'une propriété sans réglage dans le
/// panneau.
fn is_initial_value(prop: &str, v: &str) -> bool {
    let v = v.trim();
    matches!(v, "initial" | "unset" | "revert")
        || match prop {
            "font-feature-settings"
            | "font-variation-settings"
            | "font-kerning"
            | "font-variant-ligatures" => {
                matches!(v, "normal" | "auto")
            }
            "-webkit-text-stroke-width" => matches!(v, "0" | "0px"),
            "-webkit-text-stroke-color" | "text-decoration-color" => v == "currentcolor",
            "text-decoration-style" => v == "solid",
            "text-decoration-thickness" | "text-underline-offset" | "text-decoration-skip-ink" => {
                v == "auto"
            }
            "counter-increment" => v == "list-item" || v == "none",
            "counter-reset" => v == "none",
            "word-spacing" => v == "normal" || v == "0" || v == "0px",
            _ => false,
        }
}

/// Règle sans élément qui ne peut toucher aucun élément du site : classe absente du site et des
/// pages rendues, sélecteur par attribut ou identifiant sans classe, balise de formulaire absente.
fn dead_rule(
    r: &WRule,
    site_classes: &HashSet<String>,
    page_classes: &HashSet<String>,
    all_class_text: &str,
    site_tags: &HashSet<String>,
) -> bool {
    let cls = selector::classes(&r.sel);
    cls.iter().any(|c| {
        !c.starts_with("is-")
            && !site_classes.contains(c)
            && !page_classes.contains(c)
            && !all_class_text.contains(c.as_str())
    }) || (cls.is_empty() && (r.sel.contains('[') || r.sel.contains('#')))
        || selector::parse(&r.sel).is_some_and(|c| {
            c.parts
                .iter()
                .flat_map(|(_, s)| s.iter())
                .any(|t| FORM_TAGS.contains(&t.as_str()) && !site_tags.contains(t.as_str()))
        })
}

/// Report d'un élément : déclarations par niveau et canal, contenu de ses pseudo-éléments.
#[derive(Debug, Default, Clone, PartialEq)]
struct ElementPlan {
    t: BTreeMap<(usize, Channel), BTreeMap<String, String>>,
    pseudo_content: BTreeMap<String, String>,
    context: Option<String>,
}

fn flat_candidate(r: &WRule) -> bool {
    r.problem && !r.root && r.keep.is_none() && !r.matches.is_empty()
}

fn level_applies(t: &Level, here: &Level) -> bool {
    match &t.media {
        None => true,
        Some(m) => media::eval(m, here.width) == media::Eval::Yes,
    }
}

/// Canaux à calculer pour un élément : base, plus ceux des règles mises à plat qui le touchent.
fn channels_of(rules: &[WRule], on: &[usize], flat: &HashSet<usize>) -> Vec<Channel> {
    let mut set: BTreeSet<Channel> = BTreeSet::new();
    set.insert(Channel::default());
    for &ri in on {
        if !flat.contains(&ri) {
            continue;
        }
        if let Some(s) = &rules[ri].shape {
            set.insert(channel_of(s));
        }
    }
    set.into_iter().collect()
}

fn channel_of(s: &Shape) -> Channel {
    Channel {
        outer: s.outer.clone(),
        extra: s.extra.clone(),
        // Forme non reportable (survol d'un ancêtre…) : sous une condition que les niveaux ne
        // calculent pas, jamais dans l'état de base.
        state: if s.unsupported && s.state.is_none() {
            Some(":?".into())
        } else {
            s.state.clone()
        },
        pseudo: s.pseudo.clone(),
    }
}

/// Classe de l'ancêtre écrit dans la première règle mise à plat qui touche l'élément.
fn context_of(rules: &[WRule], on: &[usize], flat: &HashSet<usize>) -> Option<String> {
    on.iter()
        .filter(|ri| flat.contains(ri))
        .find_map(|&ri| rules[ri].shape.as_ref().and_then(|s| s.context.clone()))
}

/// Valeurs gagnantes d'un élément pour un niveau et un canal. `custom` : variables seulement
/// (sinon tout sauf les variables).
fn winners(
    rules: &[WRule],
    on: &[usize],
    level: &Level,
    ch: &Channel,
    keep: &dyn Fn(usize) -> bool,
    applies: &dyn Fn(&WRule, &Level) -> bool,
    custom: bool,
) -> BTreeMap<String, Win> {
    let mut out: BTreeMap<String, Win> = BTreeMap::new();
    for &ri in on {
        let r = &rules[ri];
        if !keep(ri) || !applies(r, level) {
            continue;
        }
        let rch = r.shape.as_ref().map(channel_of).unwrap_or_default();
        // Règles `[open] .x` : comptent dans le canal « ouvert » ; règles de base aussi (elles
        // s'appliquent quand l'accordéon est ouvert, le report ne garde que l'écart).
        let matches_channel = if custom {
            rch == Channel::default()
        } else {
            rch == *ch
        };
        if !matches_channel {
            continue;
        }
        let order = r.item;
        for d in &r.decls {
            if d.prop.starts_with("--") != custom {
                continue;
            }
            let key = (d.important, r.spec, order);
            match out.get(&d.prop) {
                Some((_, k)) if *k > key => {}
                _ => {
                    out.insert(d.prop.clone(), (d.value.clone(), key));
                }
            }
        }
    }
    out
}

/// Points de rupture lus dans les requêtes média : `(max-width: 1199.98px)` → 1199,
/// `(min-width: 810px)` → 809 ; entre 320 et 1600 pixels.
fn infer_breakpoints(items: &[StyleItem]) -> Vec<revel_model::Breakpoint> {
    let mut widths: BTreeSet<u32> = BTreeSet::new();
    for it in items {
        let StyleItem::Rule(r) = it else { continue };
        let Some(m) = &r.media else { continue };
        let lower = m.to_ascii_lowercase().replace(' ', "");
        for (key, adjust) in [("max-width:", 0.0), ("min-width:", -1.0)] {
            let mut rest = lower.as_str();
            while let Some(i) = rest.find(key) {
                rest = &rest[i + key.len()..];
                let num: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                let unit_em =
                    rest[num.len()..].starts_with("em") || rest[num.len()..].starts_with("rem");
                if let Ok(v) = num.parse::<f64>() {
                    let px = if unit_em { v * 16.0 } else { v };
                    let w = (px + adjust).floor();
                    if (320.0..=1600.0).contains(&w) {
                        widths.insert(w as u32);
                    }
                }
            }
        }
    }
    let mut out = vec![revel_model::Breakpoint {
        id: "desktop".into(),
        max_width: None,
        min_width: None,
    }];
    // Largeurs voisines (810 et 809 : « jusqu'à 810 » et « à partir de 810 ») : la plus petite,
    // où les deux requêtes « jusqu'à » s'appliquent et pas « à partir de 810 ».
    let mut kept: Vec<u32> = Vec::new();
    for w in widths.into_iter() {
        if kept.last().is_some_and(|k| w - k <= 2) {
            continue;
        }
        kept.push(w);
    }
    kept.reverse();
    for w in kept {
        out.push(revel_model::Breakpoint {
            id: format!("max-{w}"),
            max_width: Some(w),
            min_width: None,
        });
    }
    out
}

/// Niveaux : base (bureau), grands écrans (`min-width`), puis du plus large au plus étroit.
fn levels(bps: &[revel_model::Breakpoint]) -> Vec<Level> {
    let mut mins: Vec<u32> = bps.iter().filter_map(|b| b.min_width).collect();
    let mut maxs: Vec<u32> = bps.iter().filter_map(|b| b.max_width).collect();
    mins.sort_unstable();
    maxs.sort_unstable_by(|a, b| b.cmp(a));
    let base = mins
        .first()
        .map(|m| f64::from(*m) - 1.0)
        .unwrap_or(1440.0)
        .min(1440.0)
        .max(maxs.first().map(|m| f64::from(*m) + 1.0).unwrap_or(0.0));
    let mut out = vec![Level {
        width: base,
        media: None,
    }];
    for m in mins {
        out.push(Level {
            width: f64::from(m),
            media: Some(format!("(min-width: {m}px)")),
        });
    }
    for m in maxs {
        out.push(Level {
            width: f64::from(m),
            media: Some(format!("(max-width: {m}px)")),
        });
    }
    out
}

/// Éléments de la page, dans l'ordre du document (parents avant enfants).
fn collect(
    n: NodeRef,
    parent: Option<usize>,
    out: &mut Vec<Elem>,
    map: &mut HashMap<NodeId, usize>,
) {
    for c in n.children_it(false) {
        if c.is_element() {
            let idx = out.len();
            let inline_vars = c
                .attr("style")
                .map(|st| {
                    st.split(';')
                        .filter_map(|d| {
                            let (k, v) = d.split_once(':')?;
                            let k = k.trim().strip_prefix("--")?;
                            Some((k.to_string(), v.trim().to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            out.push(Elem {
                rv: c.attr("data-rv").map(|v| v.to_string()),
                parent,
                inline_vars,
            });
            map.insert(c.id, idx);
            collect(c, Some(idx), out, map);
        } else {
            collect(c, parent, out, map);
        }
    }
}

/// Nœud du site connu par son identifiant.
struct NodeInfo {
    owner: Owner,
    /// Noms des attributs écrits.
    attrs: HashSet<String>,
    /// Attribut `style`.
    style: Option<String>,
    tag: String,
    classes: Vec<String>,
    class_text: String,
    element: bool,
    image: bool,
    image_node: Option<revel_model::ImageNode>,
    children: usize,
}

fn node_index(site: &Site) -> HashMap<String, NodeInfo> {
    let mut out = HashMap::new();
    for (kind, id, body) in site.owners() {
        let owner = match kind {
            revel_model::ids::OwnerKind::Page => Owner::Page(id.to_string()),
            revel_model::ids::OwnerKind::Layout => Owner::Layout(id.to_string()),
            revel_model::ids::OwnerKind::Component => Owner::Component(id.to_string()),
        };
        revel_model::ids::walk(body, &mut |n| match n {
            Node::Element {
                id: Some(nid),
                tag,
                attrs,
                children,
            } => {
                let class_text = attrs
                    .iter()
                    .find(|a| a.name == "class")
                    .and_then(|a| a.value.clone())
                    .unwrap_or_default();
                out.insert(
                    nid.clone(),
                    NodeInfo {
                        owner: owner.clone(),
                        attrs: attrs.iter().map(|a| a.name.to_ascii_lowercase()).collect(),
                        style: attrs
                            .iter()
                            .find(|a| a.name == "style" && a.when.is_none())
                            .and_then(|a| a.value.clone()),
                        tag: tag.clone(),
                        classes: static_classes(&class_text),
                        class_text,
                        element: true,
                        image: false,
                        image_node: None,
                        children: children.len(),
                    },
                );
            }
            Node::Image(img) if img.id.is_some() => {
                let class_text = img.class.clone().unwrap_or_default();
                out.insert(
                    img.id.clone().unwrap_or_default(),
                    NodeInfo {
                        owner: owner.clone(),
                        attrs: ["src", "alt", "class"]
                            .iter()
                            .map(|x| x.to_string())
                            .collect(),
                        style: None,
                        tag: "img".into(),
                        classes: static_classes(&class_text),
                        class_text,
                        element: false,
                        image: true,
                        image_node: Some(img.clone()),
                        children: 0,
                    },
                );
            }
            _ => {}
        });
    }
    out
}

/// Classes écrites telles quelles (hors expressions `{{ }}` et `{% %}`).
fn static_classes(v: &str) -> Vec<String> {
    let mut s = v.to_string();
    for (a, b) in [("{{", "}}"), ("{%", "%}")] {
        while let Some(i) = s.find(a) {
            let Some(j) = s[i..].find(b) else { break };
            s.replace_range(i..i + j + b.len(), " ");
        }
    }
    s.split_whitespace().map(str::to_string).collect()
}

fn unique_name(base: &str, used: &mut HashSet<String>) -> String {
    let clean: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let clean = clean.trim_matches('-').to_string();
    let mut name = clean.clone();
    let mut k = 2;
    while used.contains(&name) {
        name = format!("{clean}-{k}");
        k += 1;
    }
    used.insert(name.clone());
    name
}

/// Texte d'une valeur `content` (`"→"`, `"\2192"`, `''`).
fn css_string(v: &str) -> String {
    let t = v.trim();
    let inner = if (t.starts_with('"') && t.ends_with('"')
        || t.starts_with('\'') && t.ends_with('\''))
        && t.len() >= 2
    {
        &t[1..t.len() - 1]
    } else {
        return String::new();
    };
    let mut out = String::new();
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let mut j = i + 1;
            let mut hex = String::new();
            while j < chars.len() && hex.len() < 6 && chars[j].is_ascii_hexdigit() {
                hex.push(chars[j]);
                j += 1;
            }
            if hex.is_empty() {
                out.push(chars[i + 1]);
                i += 2;
            } else {
                if let Some(c) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(c);
                }
                if j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                i = j;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Déclarations traduites en réglages du panneau.
fn translate_decls(decls: &[Decl], report: &mut Report) -> Vec<Decl> {
    let mut out: Vec<Decl> = Vec::new();
    for d in decls {
        if DROPPED.contains(&d.prop.as_str()) {
            report.dropped += 1;
            continue;
        }
        // Doublon préfixé d'une propriété écrite dans la même règle.
        if let Some(std_prop) = d
            .prop
            .strip_prefix("-webkit-")
            .or_else(|| d.prop.strip_prefix("-moz-"))
            && decls.iter().any(|o| o.prop == std_prop)
        {
            report.dropped += 1;
            continue;
        }
        // Débordement des deux côtés écrit séparément : `overflow` à deux valeurs (horizontal,
        // vertical). Un côté seul reste tel quel : l'autre garde la valeur d'une autre règle (un
        // report le réécrit avec elle).
        if (d.prop == "overflow-x" || d.prop == "overflow-y")
            && decls.iter().any(|o| o.prop == "overflow-x")
            && decls.iter().any(|o| o.prop == "overflow-y")
        {
            if d.prop == "overflow-y" && decls.iter().any(|o| o.prop == "overflow-x") {
                continue;
            }
            let x = decls
                .iter()
                .rev()
                .find(|o| o.prop == "overflow-x")
                .map_or("visible", |o| o.value.trim());
            let y = decls
                .iter()
                .rev()
                .find(|o| o.prop == "overflow-y")
                .map_or("visible", |o| o.value.trim());
            report.translated += 1;
            out.push(Decl {
                prop: "overflow".into(),
                value: if x == y {
                    x.to_string()
                } else {
                    format!("{x} {y}")
                },
                important: d.important,
            });
            continue;
        }
        match values::translate(&d.prop, &d.value) {
            Some(list) => {
                report.translated += 1;
                for (p, v) in list {
                    out.push(Decl {
                        prop: p,
                        value: v,
                        important: d.important,
                    });
                }
            }
            None => out.push(d.clone()),
        }
    }
    out
}

/// `box-sizing` des racines et de `*` remplacé par la fondation de Revel.
fn foundation_box_sizing(
    items: &mut Vec<StyleItem>,
    taken: &mut HashSet<String>,
    report: &mut Report,
) {
    let roots = |sel: &str| {
        let parts: Vec<String> = selector::split_list(sel)
            .into_iter()
            .map(|x| x.replace(' ', ""))
            .collect();
        !parts.is_empty()
            && parts.iter().all(|p| {
                matches!(
                    p.as_str(),
                    "html"
                        | "body"
                        | ":root"
                        | "*"
                        | "*::before"
                        | "*::after"
                        | "::before"
                        | "::after"
                        | "*:before"
                        | "*:after"
                )
            })
    };
    let mut found = false;
    for it in items.iter_mut() {
        let StyleItem::Rule(r) = it else { continue };
        if r.media.is_some() || !roots(&r.selector) || styleaudit::is_foundation(r) {
            if styleaudit::is_foundation(r) {
                found = true;
            }
            continue;
        }
        let before = r.decls.len();
        r.decls.retain(|d| {
            !(d.prop == "box-sizing" && matches!(d.value.trim(), "border-box" | "inherit"))
        });
        if r.decls.len() != before {
            found = true;
            report.translated += before - r.decls.len();
        }
    }
    if found
        && !items
            .iter()
            .any(|i| matches!(i, StyleItem::Rule(r) if styleaudit::is_foundation(r)))
    {
        let id = revel_model::ids::fresh(taken);
        taken.insert(id.clone());
        items.insert(
            0,
            StyleItem::Rule(StyleRule {
                id: Some(id),
                selector: "*, *::before, *::after".into(),
                media: None,
                decls: vec![Decl {
                    prop: "box-sizing".into(),
                    value: "border-box".into(),
                    important: false,
                }],
            }),
        );
    }
    // Avec la règle de fond, `box-sizing: border-box` ailleurs ne change rien tant qu'aucune règle
    // ne règle une autre valeur (éléments tirés des pseudo-éléments, règles gardées).
    let other = items.iter().any(|i| {
        matches!(i, StyleItem::Rule(r) if r.decls.iter().any(|d| d.prop == "box-sizing" && d.value.trim() != "border-box"))
    });
    if found && !other {
        for it in items.iter_mut() {
            let StyleItem::Rule(r) = it else { continue };
            if styleaudit::is_foundation(r) {
                continue;
            }
            let before = r.decls.len();
            r.decls.retain(|d| d.prop != "box-sizing");
            report.translated += before - r.decls.len();
        }
    }
    items.retain(|i| !matches!(i, StyleItem::Rule(r) if r.decls.is_empty()));
}

/// Conditions `@supports` vraies dans tous les navigateurs actuels : `aspect-ratio` (Chrome 88,
/// Safari 15, Firefox 89), `overflow: clip` (Chrome 90, Safari 16, Firefox 81), couleurs
/// `color()` (Chrome 111, Safari 15, Firefox 113).
const UNIVERSAL_SUPPORTS: &[&str] = &[
    "(aspect-ratio:1)",
    "(overflow:clip)",
    "(color:color(display-p31111))",
];

/// Blocs `@supports` dont la condition est toujours vraie : leurs règles reprises sans condition.
fn unwrap_supports(items: &[StyleItem], taken: &mut HashSet<String>) -> Vec<StyleItem> {
    let mut out = Vec::new();
    for it in items {
        if let StyleItem::Raw { css } = it {
            let t = css.trim();
            if let Some(rest) = t.strip_prefix("@supports")
                && let Some(open) = rest.find('{')
                && UNIVERSAL_SUPPORTS.contains(&rest[..open].replace(' ', "").as_str())
                && let Some(inner) = rest[open + 1..].trim_end().strip_suffix('}')
                && let Ok(parsed) = revel_style::parse(inner)
            {
                for mut p in parsed {
                    if let StyleItem::Rule(r) = &mut p {
                        let id = revel_model::ids::fresh(taken);
                        taken.insert(id.clone());
                        r.id = Some(id);
                    }
                    out.push(p);
                }
                continue;
            }
        }
        out.push(it.clone());
    }
    out
}

/// Variables écrites seulement dans `:root` (sans valeur sombre, ni ailleurs, ni dans un attribut),
/// qui ne sont ni des couleurs ni des tailles du site : remplacées par leur valeur.
fn substitute_root_only(site: &Site, items: &mut [StyleItem]) {
    let mut root: HashMap<String, String> = HashMap::new();
    let mut elsewhere: HashSet<String> = HashSet::new();
    for it in items.iter() {
        let StyleItem::Rule(r) = it else { continue };
        let base_root = r.selector.trim() == ":root" && r.media.is_none();
        for d in &r.decls {
            if let Some(n) = d.prop.strip_prefix("--") {
                if base_root {
                    root.insert(n.to_string(), d.value.trim().to_string());
                } else {
                    elsewhere.insert(n.to_string());
                }
            }
        }
    }
    for (_, _, body) in site.owners() {
        revel_model::ids::walk(body, &mut |n| {
            if let Node::Element { attrs, .. } | Node::Use { attrs, .. } = n {
                for a in attrs {
                    if let Some(v) = &a.value {
                        for part in v.split(';') {
                            if let Some((k, _)) = part.split_once(':')
                                && let Some(name) = k.trim().strip_prefix("--")
                            {
                                elsewhere.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        });
    }
    let only: HashMap<String, String> = root
        .into_iter()
        .filter(|(n, v)| !elsewhere.contains(n) && !styleaudit::site_value(v))
        .collect();
    if only.is_empty() {
        return;
    }
    let keep = |n: &str| !only.contains_key(n);
    for it in items.iter_mut() {
        let StyleItem::Rule(r) = it else { continue };
        if r.selector.trim() == ":root" {
            continue;
        }
        for d in r.decls.iter_mut() {
            if d.value.contains("var(")
                && let Some(v) = values::substitute(&d.value, &only, &keep)
            {
                d.value = v;
            }
        }
    }
}

/// Variables qui ne sont pas du site et ne servent plus : retirées de `:root` et des règles.
fn clean_variables(
    items: &mut Vec<StyleItem>,
    site_vars: &HashSet<String>,
    markup: &HashSet<String>,
) {
    // Bloc `@supports (…) { sélecteur { --x: … } }` qui ne déclare que des variables.
    let only_vars = regex::Regex::new(
        r"^@supports[^{]*\{\s*[^{}]+\{((?:\s*--[\w-]+\s*:[^;{}]*;?)+)\s*\}\s*\}$",
    )
    .expect("regex");
    let decl_name = regex::Regex::new(r"--([\w-]+)\s*:").expect("regex");
    loop {
        let mut used: HashSet<String> = items
            .iter()
            .filter_map(|i| match i {
                StyleItem::Rule(r) => Some(r),
                _ => None,
            })
            .flat_map(|r| r.decls.iter())
            .flat_map(|d| {
                styleaudit::var_refs(&d.value)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .collect();
        for it in items.iter() {
            if let StyleItem::Raw { css } = it {
                used.extend(styleaudit::var_refs(css).into_iter().map(str::to_string));
            }
        }
        used.extend(markup.iter().cloned());
        // Variables déclarées seulement sous condition et lues nulle part : bloc retiré.
        let n = items.len();
        items.retain(|i| match i {
            StyleItem::Raw { css } => match only_vars.captures(css.trim()) {
                Some(c) => decl_name
                    .captures_iter(&c[1])
                    .any(|d| site_vars.contains(&d[1]) || used.contains(&d[1])),
                None => true,
            },
            _ => true,
        });
        let mut changed = items.len() != n;
        for it in items.iter_mut() {
            if let StyleItem::Rule(r) = it {
                let before = r.decls.len();
                r.decls.retain(|d| {
                    let Some(n) = d.prop.strip_prefix("--") else {
                        return true;
                    };
                    styleaudit::REVEL_PROPS.contains(&d.prop.as_str())
                        || site_vars.contains(n)
                        || used.contains(n)
                });
                changed |= r.decls.len() != before;
            }
        }
        if !changed {
            break;
        }
    }
}

#[cfg(test)]
mod tests;
