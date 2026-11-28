//! Fusion d'un autre site dans le site ouvert (D-206) : pages et composants choisis, avec ce
//! qu'ils utilisent (composants, styles de leurs classes, variables, animations et polices,
//! fichiers des médias). Sert aux imports de pages (site Revel, fichiers HTML lus comme un site),
//! aux imports de composant (site Revel, fichier de composant, fichier HTML) et à l'export d'un
//! composant (fusion dans un site vide).
//!
//! Conflits : un composant identique déjà présent est réutilisé, sinon le nouveau prend un autre
//! identifiant ; une classe dont les règles diffèrent dans le site ouvert est renommée (`-2`) dans
//! ce qui est importé ; une page dont l'adresse est prise reçoit `-2`. Les règles des balises
//! (`body`, `h1`…) restent celles du site ouvert. Les éléments reçoivent de nouveaux identifiants.

use crate::Command;
use crate::make::{free_component_id, free_page_path, refresh_ids};
use revel_model::ids::{child_lists_mut, node_id_mut};
use revel_model::{Component, Decl, Node, Page, Site, StyleItem, StyleRule};
use std::collections::{BTreeMap, BTreeSet, HashSet};

/// Ce qui est importé.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Pages (identifiants dans le site source).
    pub pages: Vec<String>,
    /// Composants (identifiants dans le site source).
    pub components: Vec<String>,
    /// Composants utilisés par les pages et composants choisis, apportés s'ils manquent. Sinon
    /// une utilisation d'un composant absent du site ouvert est remplacée par son contenu.
    pub uses: bool,
    /// Styles de leurs classes, variables, animations et polices.
    pub styles: bool,
    /// Fichiers des médias, copiés dans les médias du site ouvert.
    pub media: bool,
}

/// Résultat d'une fusion : éléments à ajouter au site ouvert.
#[derive(Debug, Clone, Default)]
pub struct Merge {
    pub pages: Vec<Page>,
    pub components: Vec<Component>,
    /// Composants identiques déjà présents, réutilisés.
    pub reused: usize,
    /// Règles et blocs ajoutés à la feuille de style.
    pub styles: Vec<StyleItem>,
    /// Variables ajoutées à `:root` (requête média éventuelle : mode sombre).
    pub vars: Vec<(Option<String>, Decl)>,
    /// Fichiers des médias : adresse dans le site source → adresse dans le site ouvert.
    pub media: BTreeMap<String, String>,
    /// Classes renommées (avant, après).
    pub renamed: Vec<(String, String)>,
}

impl Selection {
    /// Tout ce que la sélection utilise (composants, styles, médias) ou rien.
    pub fn all(pages: Vec<String>, components: Vec<String>, bring: bool) -> Self {
        Self {
            pages,
            components,
            uses: bring,
            styles: bring,
            media: bring,
        }
    }
}

impl Merge {
    /// Rien à ajouter.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
            && self.components.is_empty()
            && self.styles.is_empty()
            && self.vars.is_empty()
    }
}

/// Classes d'un sélecteur (`.a:hover .b > p` → `a`, `b`).
pub fn selector_classes(sel: &str) -> Vec<String> {
    let b = sel.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'.' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-' || b[j] == b'_') {
                j += 1;
            }
            if j > start && !b[start].is_ascii_digit() {
                out.push(sel[start..j].to_string());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Parties d'un sélecteur multiple (virgules hors parenthèses : `:is(.a, .b)` reste entier).
fn split_selector(sel: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in sel.char_indices() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(sel[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(sel[start..].trim());
    out.retain(|p| !p.is_empty());
    out
}

/// Remplace la classe `from` par `to` dans un sélecteur (jetons entiers seulement).
fn rename_in_selector(sel: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(sel.len());
    let b = sel.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'.' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'-' || b[j] == b'_') {
                j += 1;
            }
            out.push('.');
            out.push_str(if &sel[start..j] == from {
                to
            } else {
                &sel[start..j]
            });
            i = j;
        } else {
            let c = sel[i..].chars().next().unwrap_or(' ');
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// Classe d'état ajoutée par une interaction (`is-open`, `has-x`) : pas une classe à part.
fn is_state(c: &str) -> bool {
    c.starts_with("is-") || c.starts_with("has-") || c.starts_with("w--")
}

/// Visite mutable de tous les nœuds d'un arbre.
fn each_mut(nodes: &mut [Node], f: &mut dyn FnMut(&mut Node)) {
    for n in nodes {
        f(n);
        for list in child_lists_mut(n) {
            each_mut(list, f);
        }
    }
}

/// Classes portées par les éléments d'un arbre.
fn classes_in(nodes: &[Node], out: &mut BTreeSet<String>) {
    revel_model::ids::walk(nodes, &mut |n| match n {
        Node::Element { attrs, .. } | Node::Use { attrs, .. } => {
            for a in attrs.iter().filter(|a| a.name == "class") {
                for c in a.value.as_deref().unwrap_or("").split_whitespace() {
                    if !c.contains("{{") {
                        out.insert(c.to_string());
                    }
                }
            }
        }
        Node::Image(img) => {
            for c in img.class.as_deref().unwrap_or("").split_whitespace() {
                out.insert(c.to_string());
            }
        }
        _ => {}
    });
}

/// Renomme une classe partout dans un arbre (attribut `class`, image, références des
/// interactions `.classe` dans les attributs `data-revel-*`).
fn rename_class_in(nodes: &mut [Node], from: &str, to: &str) {
    let swap = |v: &str| -> String {
        v.split(' ')
            .map(|c| if c == from { to } else { c })
            .collect::<Vec<_>>()
            .join(" ")
    };
    each_mut(nodes, &mut |n| match n {
        Node::Element { attrs, .. } | Node::Use { attrs, .. } => {
            for a in attrs.iter_mut() {
                let Some(v) = a.value.as_mut() else { continue };
                if a.name == "class" {
                    *v = swap(v);
                } else if a.name.starts_with("data-revel") && v.contains(&format!(".{from}")) {
                    *v = rename_in_selector(v, from, to);
                }
            }
        }
        Node::Image(img) => {
            if let Some(c) = img.class.as_mut() {
                *c = swap(c);
            }
        }
        _ => {}
    });
}

/// Remplace les adresses des médias dans un arbre (attributs, images) : correspondance exacte
/// ou dans une liste (`srcset`, `url(…)` d'un style écrit dans l'élément).
fn swap_urls(text: &str, map: &BTreeMap<String, String>) -> Option<String> {
    let mut out = text.to_string();
    let mut changed = false;
    // Les plus longues d'abord (`/media/a.jpg.webp` avant `/media/a.jpg`).
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_by_key(|k| std::cmp::Reverse(k.len()));
    for k in keys {
        if out.contains(k.as_str()) {
            out = out.replace(k.as_str(), &map[k]);
            changed = true;
        }
    }
    changed.then_some(out)
}

fn urls_in_tree(nodes: &mut [Node], map: &BTreeMap<String, String>) {
    each_mut(nodes, &mut |n| match n {
        Node::Element { .. } | Node::Use { .. } => {
            if let Node::Use { args, .. } = n {
                for v in args.values_mut() {
                    if let Some(s) = swap_urls(v, map) {
                        *v = s;
                    }
                }
            }
            let (Node::Element { attrs, .. } | Node::Use { attrs, .. }) = n else {
                return;
            };
            for a in attrs.iter_mut() {
                if let Some(v) = a.value.as_mut()
                    && let Some(s) = swap_urls(v, map)
                {
                    *v = s;
                }
            }
        }
        Node::Image(img) => {
            if let Some(s) = swap_urls(&img.src, map) {
                img.src = s;
            }
        }
        _ => {}
    });
}

/// Adresses de fichiers locaux dans un texte (`/dossier/fichier.ext`), pour les médias.
fn local_urls(text: &str, out: &mut BTreeSet<String>) {
    for part in text.split(|c: char| c.is_whitespace() || "\"'(),".contains(c)) {
        let p = part.trim();
        if p.starts_with('/')
            && !p.starts_with("//")
            && p.rsplit('/').next().is_some_and(|f| f.contains('.'))
            && !p.contains("{{")
        {
            out.insert(p.split(['?', '#']).next().unwrap_or(p).to_string());
        }
    }
}

fn urls_of_tree(nodes: &[Node], out: &mut BTreeSet<String>) {
    revel_model::ids::walk(nodes, &mut |n| match n {
        Node::Element { attrs, .. } | Node::Use { attrs, .. } => {
            if let Node::Use { args, .. } = n {
                for v in args.values() {
                    local_urls(v, out);
                }
            }
            for a in attrs {
                if let Some(v) = &a.value {
                    local_urls(v, out);
                }
            }
        }
        Node::Image(img) => local_urls(&img.src, out),
        _ => {}
    });
}

/// Noms de variables CSS lus par une valeur (`var(--x)`).
fn vars_in(value: &str, out: &mut BTreeSet<String>) {
    let mut rest = value;
    while let Some(i) = rest.find("var(--") {
        let s = &rest[i + 4..];
        let end = s
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(s.len());
        out.insert(s[..end].to_string());
        rest = &s[end..];
    }
}

fn rules(site: &Site) -> Vec<&StyleRule> {
    site.style_rules().collect()
}

/// Empreinte d'une règle (sélecteur, média, déclarations) pour comparer deux sites.
fn print(r: &StyleRule) -> String {
    let d: Vec<String> = r
        .decls
        .iter()
        .map(|d| {
            format!(
                "{}:{}{}",
                d.prop,
                d.value.trim(),
                if d.important { "!" } else { "" }
            )
        })
        .collect();
    format!(
        "{}@{}{{{}}}",
        r.selector,
        r.media.as_deref().unwrap_or(""),
        d.join(";")
    )
}

/// Contenu comparable d'un composant (sans les identifiants des nœuds).
fn body_print(nodes: &[Node]) -> String {
    let mut v = nodes.to_vec();
    each_mut(&mut v, &mut |n| {
        if let Some(id) = node_id_mut(n) {
            *id = None;
        }
    });
    serde_json::to_string(&v).unwrap_or_default()
}

/// Composants utilisés par un arbre, directement ou non (dans le site source).
fn used_components(source: &Site, nodes: &[Node], out: &mut BTreeSet<String>) {
    let mut todo = Vec::new();
    revel_model::ids::walk(nodes, &mut |n| {
        if let Node::Use { component, .. } = n {
            todo.push(component.clone());
        }
    });
    while let Some(c) = todo.pop() {
        if !out.insert(c.clone()) {
            continue;
        }
        if let Some(comp) = source.component(&c) {
            revel_model::ids::walk(&comp.body, &mut |n| {
                if let Node::Use { component, .. } = n {
                    todo.push(component.clone());
                }
            });
        }
    }
}

/// Remplace les utilisations des composants absents du site ouvert par leur contenu (valeurs
/// des propriétés écrites dedans), pour un import sans « ce qu'ils utilisent ».
fn inline_missing(nodes: &mut Vec<Node>, source: &Site, target: &Site, depth: usize) {
    if depth > 16 {
        return;
    }
    let mut out = Vec::with_capacity(nodes.len());
    for mut n in std::mem::take(nodes) {
        if let Node::Use {
            component,
            args,
            variant,
            attrs,
            children,
            ..
        } = &n
            && target.component(component).is_none()
            && let Some(c) = source.component(component)
        {
            let mut body = crate::make::expanded_use(c, args, variant.as_deref(), attrs, children);
            inline_missing(&mut body, source, target, depth + 1);
            out.extend(body);
            continue;
        }
        for list in child_lists_mut(&mut n) {
            inline_missing(list, source, target, depth);
        }
        out.push(n);
    }
    *nodes = out;
}

/// Fusionne la sélection de `source` dans `target`. `place` donne l'adresse dans le site ouvert
/// d'un fichier local du site source (`None` : pas un média, laissé tel quel).
pub fn merge(
    target: &Site,
    source: &Site,
    sel: &Selection,
    place: &mut dyn FnMut(&str) -> Option<String>,
) -> Merge {
    let mut m = Merge::default();
    // Site de travail : ce qui est ajouté y entre au fur et à mesure (adresses et identifiants
    // libres calculés avec lui).
    let mut work = target.clone();
    let mut taken = work.node_ids();
    for i in source.node_ids() {
        taken.insert(i);
    }

    // Pages et composants choisis, tels quels.
    let mut pages: Vec<Page> = sel
        .pages
        .iter()
        .filter_map(|id| source.pages.iter().find(|p| &p.id == id).cloned())
        .collect();
    let mut wanted: BTreeSet<String> = sel.components.iter().cloned().collect();
    if sel.uses {
        let mut extra = BTreeSet::new();
        for p in &pages {
            used_components(source, &p.body, &mut extra);
        }
        for c in &sel.components {
            if let Some(comp) = source.component(c) {
                used_components(source, &comp.body, &mut extra);
            }
        }
        wanted.extend(extra);
    } else {
        for p in &mut pages {
            inline_missing(&mut p.body, source, target, 0);
        }
    }
    let mut comps: Vec<Component> = source
        .components
        .iter()
        .filter(|c| wanted.contains(&c.id))
        .cloned()
        .collect();
    if !sel.uses {
        for c in &mut comps {
            inline_missing(&mut c.body, source, target, 0);
        }
    }

    // Composants : identique présent → réutilisé ; sinon nouvel identifiant si pris.
    let mut comp_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut keep: Vec<Component> = Vec::new();
    for c in comps {
        match work.component(&c.id) {
            Some(t) if body_print(&t.body) == body_print(&c.body) && t.props == c.props => {
                m.reused += 1;
            }
            Some(_) => {
                let id = free_component_id(&work, &c.id);
                comp_ids.insert(c.id.clone(), id.clone());
                let mut c2 = c;
                c2.id = id;
                work.components.push(c2.clone());
                keep.push(c2);
            }
            None => {
                work.components.push(c.clone());
                keep.push(c);
            }
        }
    }
    let retarget = |nodes: &mut Vec<Node>| {
        each_mut(nodes, &mut |n| {
            if let Node::Use { component, .. } = n
                && let Some(to) = comp_ids.get(component)
            {
                *component = to.clone();
            }
        });
    };
    for c in &mut keep {
        retarget(&mut c.body);
    }
    for p in &mut pages {
        retarget(&mut p.body);
    }

    // Styles : règles des classes portées par ce qui est importé.
    let mut used = BTreeSet::new();
    for p in &pages {
        classes_in(&p.body, &mut used);
    }
    for c in &keep {
        classes_in(&c.body, &mut used);
    }
    let src_rules = rules(source);
    let tgt_rules = rules(target);
    // Partie d'un sélecteur qui vise ce qui est importé : ses classes y sont toutes (ou sont des
    // états) ; un sélecteur multiple ne garde que ces parties.
    let wanted = |part: &str| -> bool {
        let cs = selector_classes(part);
        cs.iter().any(|c| used.contains(c)) && cs.iter().all(|c| used.contains(c) || is_state(c))
    };
    let relevant = |r: &StyleRule| -> Option<StyleRule> {
        let parts: Vec<&str> = split_selector(&r.selector)
            .into_iter()
            .filter(|p| wanted(p))
            .collect();
        (!parts.is_empty()).then(|| StyleRule {
            selector: parts.join(", "),
            ..r.clone()
        })
    };
    let mut copied: Vec<StyleRule> = Vec::new();
    if sel.styles {
        copied = src_rules.iter().filter_map(|r| relevant(r)).collect();
        // Classe en conflit : règles différentes dans le site ouvert → renommée.
        let mut tgt_classes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for r in &tgt_rules {
            for c in selector_classes(&r.selector) {
                tgt_classes.entry(c).or_default().insert(print(r));
            }
        }
        let mut all_classes: HashSet<String> = tgt_classes.keys().cloned().collect();
        for r in &src_rules {
            all_classes.extend(selector_classes(&r.selector));
        }
        let mut renames: Vec<(String, String)> = Vec::new();
        for c in &used {
            let Some(theirs) = tgt_classes.get(c) else {
                continue;
            };
            let ours: BTreeSet<String> = copied
                .iter()
                .filter(|r| selector_classes(&r.selector).contains(c))
                .map(print)
                .collect();
            if ours.is_empty() || ours.is_subset(theirs) {
                continue;
            }
            let mut n = 2;
            let mut to = format!("{c}-{n}");
            while all_classes.contains(&to) {
                n += 1;
                to = format!("{c}-{n}");
            }
            all_classes.insert(to.clone());
            renames.push((c.clone(), to));
        }
        for (from, to) in &renames {
            for r in &mut copied {
                r.selector = rename_in_selector(&r.selector, from, to);
            }
            for p in &mut pages {
                rename_class_in(&mut p.body, from, to);
            }
            for c in &mut keep {
                rename_class_in(&mut c.body, from, to);
            }
        }
        m.renamed = renames;
        // Règles déjà présentes à l'identique : pas recopiées.
        let present: HashSet<String> = tgt_rules.iter().map(|r| print(r)).collect();
        copied.retain(|r| !present.contains(&print(r)));
    }

    // Variables, animations et polices lues par les règles copiées.
    if sel.styles {
        let mut vars = BTreeSet::new();
        for r in &copied {
            for d in &r.decls {
                vars_in(&d.value, &mut vars);
            }
        }
        revel_model::ids::walk(
            &pages
                .iter()
                .flat_map(|p| p.body.clone())
                .collect::<Vec<_>>(),
            &mut |n| {
                if let Node::Element { attrs, .. } = n {
                    for a in attrs.iter().filter(|a| a.name == "style") {
                        vars_in(a.value.as_deref().unwrap_or(""), &mut vars);
                    }
                }
            },
        );
        let root_of = |s: &Site| -> Vec<(Option<String>, Decl)> {
            s.style_rules()
                .filter(|r| r.selector.trim() == ":root")
                .flat_map(|r| {
                    r.decls
                        .iter()
                        .map(|d| (r.media.clone(), d.clone()))
                        .collect::<Vec<_>>()
                })
                .collect()
        };
        let src_root = root_of(source);
        let tgt_root = root_of(target);
        let mut todo: Vec<String> = vars.into_iter().collect();
        let mut seen = BTreeSet::new();
        while let Some(v) = todo.pop() {
            if !seen.insert(v.clone()) || tgt_root.iter().any(|(_, d)| d.prop == v) {
                continue;
            }
            for (media, d) in src_root.iter().filter(|(_, d)| d.prop == v) {
                let mut more = BTreeSet::new();
                vars_in(&d.value, &mut more);
                todo.extend(more);
                m.vars.push((media.clone(), d.clone()));
            }
        }
        // `@keyframes` des animations et `@font-face` des familles, absents du site ouvert.
        let mut names = BTreeSet::new();
        let mut families = BTreeSet::new();
        for r in &copied {
            for d in &r.decls {
                if d.prop == "animation" || d.prop == "animation-name" {
                    for w in d.value.split([' ', ',']) {
                        names.insert(w.trim().to_string());
                    }
                }
                if d.prop == "font-family" || d.prop == "font" {
                    for w in d.value.split(',') {
                        families.insert(w.trim().trim_matches(['"', '\'']).to_ascii_lowercase());
                    }
                }
            }
        }
        let raws = |s: &Site| -> Vec<String> {
            s.stylesheet
                .as_ref()
                .map(|st| {
                    st.items
                        .iter()
                        .filter_map(|i| match i {
                            StyleItem::Raw { css } => Some(css.clone()),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let tgt_raw = raws(target).join("\n").to_ascii_lowercase();
        for css in raws(source) {
            let l = css.to_ascii_lowercase();
            let anim = l.trim_start().starts_with("@keyframes")
                && names.iter().any(|n| {
                    !n.is_empty() && l.contains(&format!("@keyframes {}", n.to_ascii_lowercase()))
                });
            let font = l.trim_start().starts_with("@font-face")
                && families
                    .iter()
                    .any(|f| !f.is_empty() && l.contains(f.as_str()));
            if (anim || font) && !tgt_raw.contains(&l) {
                m.styles.push(StyleItem::Raw { css });
            }
        }
    }
    for mut r in copied {
        r.id = Some(revel_model::ids::fresh(&taken));
        taken.insert(r.id.clone().unwrap_or_default());
        m.styles.push(StyleItem::Rule(r));
    }

    // Médias : fichiers locaux lus par ce qui est importé (et par les styles copiés).
    if sel.media {
        let mut urls = BTreeSet::new();
        for p in &pages {
            urls_of_tree(&p.body, &mut urls);
            for v in p.props.values() {
                if let Some(s) = v.as_str() {
                    local_urls(s, &mut urls);
                }
            }
        }
        for c in &keep {
            urls_of_tree(&c.body, &mut urls);
            for p in &c.props {
                if let Some(s) = p.default.as_ref().and_then(|v| v.as_str()) {
                    local_urls(s, &mut urls);
                }
            }
        }
        for i in &m.styles {
            match i {
                StyleItem::Rule(r) => {
                    for d in &r.decls {
                        local_urls(&d.value, &mut urls);
                    }
                }
                StyleItem::Raw { css } => local_urls(css, &mut urls),
                _ => {}
            }
        }
        for u in urls {
            if let Some(to) = place(&u) {
                m.media.insert(u, to);
            }
        }
        let map: BTreeMap<String, String> = m
            .media
            .iter()
            .filter(|(a, b)| a != b)
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        if !map.is_empty() {
            for p in &mut pages {
                urls_in_tree(&mut p.body, &map);
                for v in p.props.values_mut() {
                    if let Some(s) = v.as_str().and_then(|s| swap_urls(s, &map)) {
                        *v = serde_json::Value::String(s);
                    }
                }
            }
            for c in &mut keep {
                urls_in_tree(&mut c.body, &map);
                for p in &mut c.props {
                    if let Some(s) = p
                        .default
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .and_then(|s| swap_urls(s, &map))
                    {
                        p.default = Some(serde_json::Value::String(s));
                    }
                }
            }
            for i in &mut m.styles {
                match i {
                    StyleItem::Rule(r) => {
                        for d in &mut r.decls {
                            if let Some(s) = swap_urls(&d.value, &map) {
                                d.value = s;
                            }
                        }
                    }
                    StyleItem::Raw { css } => {
                        if let Some(s) = swap_urls(css, &map) {
                            *css = s;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Nouveaux identifiants des éléments ; pages à des adresses libres.
    for c in &mut keep {
        for n in c.body.iter_mut() {
            refresh_ids(n, &mut taken);
        }
    }
    let layout = crate::make::usual_layout(&work);
    for mut p in pages {
        for n in p.body.iter_mut() {
            refresh_ids(n, &mut taken);
        }
        let trimmed = p.path.trim_end_matches('/');
        let (folder, last) = trimmed.rsplit_once('/').unwrap_or(("", trimmed));
        // Page d'accueil importée : nommée d'après son nom ou son titre.
        let slug = match last {
            "" => {
                let title = p
                    .name
                    .clone()
                    .or_else(|| {
                        p.props
                            .get("seo")
                            .and_then(|s| s.get("title"))
                            .or_else(|| p.props.get("title"))
                            .and_then(|t| t.as_str())
                            .filter(|t| !t.contains("{{"))
                            .map(str::to_string)
                    })
                    .unwrap_or_default();
                match revel_model::slugify(&title) {
                    s if s.is_empty() => revel_model::slugify(&p.id),
                    s => s,
                }
            }
            l => l.to_string(),
        };
        p.path = free_page_path(&work, folder, &slug);
        p.id = crate::make::free_page_id(&work, &p.id);
        if p.layout
            .as_ref()
            .is_some_and(|l| !work.layouts.iter().any(|x| &x.id == l))
        {
            p.layout = layout.clone();
        }
        work.pages.push(p.clone());
        m.pages.push(p);
    }
    m.components = keep;
    m
}

/// Commandes qui ajoutent la fusion au site ouvert (une seule étape d'annulation).
pub fn commands(target: &Site, m: &Merge) -> Vec<Command> {
    let mut out: Vec<Command> = Vec::new();
    for c in &m.components {
        out.push(Command::AddComponent {
            component: c.clone(),
            index: None,
        });
    }
    let mut n = target
        .stylesheet
        .as_ref()
        .map(|s| s.items.len())
        .unwrap_or(0);
    // Variables : dans la règle `:root` de même média, sinon dans une nouvelle.
    let mut taken = target.node_ids();
    let mut new_roots: Vec<(Option<String>, Vec<Decl>)> = Vec::new();
    for (media, d) in &m.vars {
        let rule = target
            .style_rules()
            .find(|r| r.selector.trim() == ":root" && r.media == *media)
            .and_then(|r| Some((r.id.clone()?, r.decls.len())));
        match rule {
            Some((id, len)) => {
                let extra = out
                    .iter()
                    .filter(|c| matches!(c, Command::InsertDecl { rule, .. } if *rule == id))
                    .count();
                out.push(Command::InsertDecl {
                    rule: id,
                    index: len + extra,
                    decl: d.clone(),
                });
            }
            None => match new_roots.iter_mut().find(|(m, _)| m == media) {
                Some((_, v)) => v.push(d.clone()),
                None => new_roots.push((media.clone(), vec![d.clone()])),
            },
        }
    }
    let mut items: Vec<StyleItem> = new_roots
        .into_iter()
        .map(|(media, decls)| {
            let id = revel_model::ids::fresh(&taken);
            taken.insert(id.clone());
            StyleItem::Rule(StyleRule {
                id: Some(id),
                selector: ":root".into(),
                media,
                decls,
            })
        })
        .collect();
    items.extend(m.styles.iter().cloned());
    // `@font-face` et `@keyframes` avant les règles.
    items.sort_by_key(|i| !matches!(i, StyleItem::Raw { .. }));
    for item in items {
        out.push(Command::InsertStyle { index: n, item });
        n += 1;
    }
    for p in &m.pages {
        out.push(Command::AddPage {
            page: p.clone(),
            index: None,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use revel_model::{Attr, Stylesheet};

    fn el(id: &str, tag: &str, class: &str, children: Vec<Node>) -> Node {
        Node::Element {
            id: Some(id.into()),
            tag: tag.into(),
            attrs: vec![Attr {
                name: "class".into(),
                value: Some(class.into()),
                when: None,
            }],
            children,
        }
    }

    fn rule(id: &str, sel: &str, decls: &[(&str, &str)]) -> StyleItem {
        StyleItem::Rule(StyleRule {
            id: Some(id.into()),
            selector: sel.into(),
            media: None,
            decls: decls
                .iter()
                .map(|(p, v)| Decl {
                    prop: p.to_string(),
                    value: v.to_string(),
                    important: false,
                })
                .collect(),
        })
    }

    fn page(id: &str, path: &str, body: Vec<Node>) -> Page {
        Page {
            id: id.into(),
            path: path.into(),
            layout: None,
            props: Default::default(),
            computed: Default::default(),
            body,
            draft: false,
            name: None,
            code: Default::default(),
        }
    }

    fn site(items: Vec<StyleItem>) -> Site {
        let mut s = Site::new("Essai", "fr");
        s.stylesheet = Some(Stylesheet {
            url: "/s.css".into(),
            source: None,
            items,
        });
        s
    }

    #[test]
    fn classes_du_selecteur() {
        assert_eq!(
            selector_classes(".a:hover .b-c > p.d_e"),
            vec!["a", "b-c", "d_e"]
        );
        assert_eq!(
            split_selector(".a, :is(.b, .c) p"),
            vec![".a", ":is(.b, .c) p"]
        );
        assert_eq!(
            rename_in_selector(".carte .carte-titre", "carte", "carte-2"),
            ".carte-2 .carte-titre"
        );
    }

    #[test]
    fn fusion_pages_composants_styles_medias() {
        let mut src = site(vec![
            rule(
                "s1",
                ":root",
                &[
                    ("--accent", "#f00"),
                    ("--pas", "var(--base)"),
                    ("--base", "8px"),
                ],
            ),
            rule(
                "s2",
                ".carte",
                &[("color", "var(--accent)"), ("padding", "var(--pas)")],
            ),
            rule("s3", ".titre", &[("font-size", "24px")]),
            rule("s4", ".carte:hover .titre", &[("color", "red")]),
            rule("s5", ".ailleurs", &[("color", "blue")]),
            rule("s6", "h1", &[("margin", "0")]),
            rule("s7", ".fond", &[("background", "url(/media/fond.jpg)")]),
        ]);
        src.components.push(Component::new(
            "carte",
            vec![el(
                "c1",
                "div",
                "carte fond",
                vec![el("c2", "h2", "titre", vec![])],
            )],
        ));
        src.pages.push(page(
            "services",
            "/services/",
            vec![
                el(
                    "p1",
                    "section",
                    "titre",
                    vec![Node::Use {
                        id: Some("u1".into()),
                        component: "carte".into(),
                        args: Default::default(),
                        variant: None,
                        attrs: vec![],
                        children: vec![],
                    }],
                ),
                Node::Element {
                    id: Some("p2".into()),
                    tag: "img".into(),
                    attrs: vec![Attr {
                        name: "src".into(),
                        value: Some("/media/photo.jpg".into()),
                        when: None,
                    }],
                    children: vec![],
                },
            ],
        ));
        // Site ouvert : `.titre` différente, page `/services/` prise, composant `carte` absent.
        let mut tgt = site(vec![
            rule("t1", ".titre", &[("font-size", "40px")]),
            rule("t2", ":root", &[("--accent", "#00f")]),
        ]);
        tgt.pages.push(page("services", "/services/", vec![]));
        let sel = Selection {
            pages: vec!["services".into()],
            components: vec![],
            uses: true,
            styles: true,
            media: true,
        };
        let m = merge(&tgt, &src, &sel, &mut |u| {
            u.starts_with("/media/")
                .then(|| u.replace("/media/", "/media/import-"))
        });
        assert_eq!(m.pages.len(), 1);
        assert_eq!(m.pages[0].path, "/services-2/");
        assert_ne!(m.pages[0].id, "services");
        assert_eq!(m.components.len(), 1, "composant utilisé apporté");
        assert_eq!(
            m.renamed,
            vec![("titre".to_string(), "titre-2".to_string())]
        );
        let sels: Vec<String> = m
            .styles
            .iter()
            .filter_map(|i| match i {
                StyleItem::Rule(r) => Some(r.selector.clone()),
                _ => None,
            })
            .collect();
        assert!(sels.contains(&".carte".to_string()));
        assert!(sels.contains(&".titre-2".to_string()));
        assert!(sels.contains(&".carte:hover .titre-2".to_string()));
        assert!(!sels.iter().any(|s| s == ".ailleurs" || s == "h1"));
        // Variables manquantes (et celles qu'elles lisent) ; --accent déjà là.
        let vars: Vec<&str> = m.vars.iter().map(|(_, d)| d.prop.as_str()).collect();
        assert!(vars.contains(&"--pas") && vars.contains(&"--base") && !vars.contains(&"--accent"));
        // Médias : page et règle du composant.
        assert_eq!(
            m.media.get("/media/photo.jpg").map(String::as_str),
            Some("/media/import-photo.jpg")
        );
        assert!(m.media.contains_key("/media/fond.jpg"));
        let page = serde_json::to_string(&m.pages[0]).unwrap();
        assert!(page.contains("/media/import-photo.jpg") && page.contains("titre-2"));
        // Nouveaux identifiants.
        assert!(!page.contains("\"p1\""));
        let cmds = commands(&tgt, &m);
        assert!(matches!(cmds.first(), Some(Command::AddComponent { .. })));
        assert!(matches!(cmds.last(), Some(Command::AddPage { .. })));
        assert!(
            cmds.iter()
                .any(|c| matches!(c, Command::InsertDecl { rule, .. } if rule == "t2"))
        );
    }

    #[test]
    fn composant_identique_reutilise_sinon_renomme() {
        let comp = Component::new("bouton", vec![el("b1", "a", "bouton", vec![])]);
        let mut src = site(vec![rule("s1", ".bouton", &[("color", "red")])]);
        src.components.push(comp.clone());
        let mut tgt = site(vec![rule("t1", ".bouton", &[("color", "red")])]);
        let mut same = comp.clone();
        if let Node::Element { id, .. } = &mut same.body[0] {
            *id = Some("autre".into());
        }
        tgt.components.push(same);
        let sel = Selection {
            pages: vec![],
            components: vec!["bouton".into()],
            uses: true,
            styles: true,
            media: true,
        };
        let m = merge(&tgt, &src, &sel, &mut |_| None);
        assert_eq!((m.reused, m.components.len(), m.styles.len()), (1, 0, 0));
        // Contenu différent : nouvel identifiant.
        tgt.components[0].body = vec![el("x", "button", "bouton", vec![])];
        let m = merge(&tgt, &src, &sel, &mut |_| None);
        assert_eq!(m.components.len(), 1);
        assert_eq!(m.components[0].id, "bouton-2");
        assert!(m.renamed.is_empty(), "même règle : classe gardée");
    }

    #[test]
    fn sans_ce_qu_elles_utilisent() {
        let mut src = site(vec![rule("s1", ".carte", &[("color", "red")])]);
        src.components.push(Component::new(
            "carte",
            vec![el("c1", "div", "carte", vec![])],
        ));
        src.pages.push(page(
            "a",
            "/a/",
            vec![Node::Use {
                id: Some("u".into()),
                component: "carte".into(),
                args: Default::default(),
                variant: None,
                attrs: vec![],
                children: vec![],
            }],
        ));
        let tgt = site(vec![]);
        let m = merge(
            &tgt,
            &src,
            &Selection {
                pages: vec!["a".into()],
                components: vec![],
                uses: false,
                styles: false,
                media: false,
            },
            &mut |_| Some("/x".into()),
        );
        assert!(m.components.is_empty() && m.styles.is_empty() && m.media.is_empty());
        assert!(
            matches!(&m.pages[0].body[0], Node::Element { tag, .. } if tag == "div"),
            "contenu du composant mis à sa place"
        );
    }
}
