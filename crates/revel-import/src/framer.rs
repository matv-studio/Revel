//! Framer (D-109) : ce que la page publiée ne contient pas, lu dans les modules du site.
//!
//! La page servie par Framer n'est que le premier rendu de son programme React : le reste vient
//! des modules (`*.mjs`) et des données du CMS (`*.framercms`). Sans exécuter ce programme,
//! on y lit :
//! - les **effets d'apparition au défilement** de chaque élément (`__framer__enter`, transition),
//!   repris en animations Revel (`data-revel-appear`) ;
//! - les **états de survol** des composants (`"<variante>-hover": {…}`, classes `.hover` de la
//!   feuille de style), repris en règles `:hover` ;
//! - les **composants de code** reconnus : barre de progression du défilement (`ScrollProgress`)
//!   et défilement doux (`Lenis`) ;
//! - les **collections du CMS** : fichiers `.framercms` (format binaire décodé ci-dessous), une
//!   collection par langue, avec le schéma des champs écrit dans le module de la collection.
//!
//! Ce qui n'existe que dans le programme (contenu d'une variante affichée au clic : menu
//! déplié…) n'est pas dans la page : signalé dans le bilan.
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

// -------------------------------------------------------------------------------------------------
// Littéraux JavaScript (modules compilés de Framer)

/// Fin (exclue) du littéral qui commence à `start` (`{`, `[`, `(`, chaîne) : chaînes et
/// imbrications suivies.
pub(crate) fn literal_end(s: &str, start: usize) -> Option<usize> {
    let b = s.as_bytes();
    let first = *b.get(start)?;
    if matches!(first, b'"' | b'\'' | b'`') {
        return string_end(b, start);
    }
    let mut depth = 0i32;
    let mut i = start;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' | b'`' => {
                i = string_end(b, i)?;
                continue;
            }
            b'{' | b'[' | b'(' => depth += 1,
            b'}' | b']' | b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn string_end(b: &[u8], start: usize) -> Option<usize> {
    let q = b[start];
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            c if c == q => return Some(i + 1),
            // Gabarit `${…}` : imbrication suivie.
            b'$' if q == b'`' && b.get(i + 1) == Some(&b'{') => {
                i = literal_end(std::str::from_utf8(b).ok()?, i + 1)?;
            }
            _ => i += 1,
        }
    }
    None
}

/// Entrées de premier niveau d'un objet littéral `{a:1,"b-c":`x`,d:{…}}` : (clé, valeur brute).
pub(crate) fn object_entries(obj: &str) -> Vec<(String, String)> {
    let inner = obj
        .trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or("");
    let mut out = Vec::new();
    let b = inner.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // Clé.
        let key_start = i;
        let key_end;
        let key = if matches!(b[i], b'"' | b'\'' | b'`') {
            key_end = match string_end(b, i) {
                Some(e) => e,
                None => break,
            };
            inner[key_start + 1..key_end - 1].to_string()
        } else {
            let mut j = i;
            while j < b.len() && b[j] != b':' && b[j] != b',' {
                j += 1;
            }
            key_end = j;
            inner[key_start..key_end].trim().to_string()
        };
        i = key_end;
        if i >= b.len() || b[i] == b',' {
            // Raccourci `{a,b}` : la valeur est la variable du même nom.
            if !key.is_empty() {
                out.push((key.clone(), key));
            }
            i += 1;
            continue;
        }
        i += 1; // ':'
        let v_start = i;
        while i < b.len() && b[i] != b',' {
            if matches!(b[i], b'"' | b'\'' | b'`' | b'{' | b'[' | b'(') {
                match literal_end(inner, i) {
                    Some(e) => i = e,
                    None => return out,
                }
            } else {
                i += 1;
            }
        }
        out.push((key, inner[v_start..i].trim().to_string()));
        i += 1;
    }
    out
}

/// Valeur simple : nombre, booléen (`!0`, `!1`), chaîne sans substitution.
pub(crate) fn scalar(v: &str) -> Option<Value> {
    let v = v.trim();
    match v {
        "!0" | "true" => return Some(Value::Bool(true)),
        "!1" | "false" => return Some(Value::Bool(false)),
        "null" | "void 0" => return Some(Value::Null),
        _ => {}
    }
    if let Some(s) = v
        .strip_prefix('`')
        .and_then(|s| s.strip_suffix('`'))
        .filter(|s| !s.contains("${"))
        .or_else(|| v.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
        .or_else(|| v.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
    {
        return Some(Value::String(s.replace("\\\"", "\"")));
    }
    let n = v.strip_prefix('-').unwrap_or(v);
    if !n.is_empty()
        && n.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == 'e')
    {
        let mut t = v.to_string();
        if t.starts_with('.') {
            t.insert(0, '0');
        } else if t.starts_with("-.") {
            t.insert(1, '0');
        }
        return t.parse::<f64>().ok().map(|f| json!(f));
    }
    None
}

/// Objets littéraux affectés à une variable dans un module (`W={bounce:.1,duration:1.5}`).
pub(crate) fn object_vars(js: &str) -> HashMap<String, String> {
    let re = regex::Regex::new(r"(?:^|[,;{(\s])([A-Za-z_$][\w$]*)=\{").expect("regex");
    let mut out = HashMap::new();
    for m in re.captures_iter(js) {
        let open = m.get(0).expect("m").end() - 1;
        if let Some(end) = literal_end(js, open) {
            out.entry(m[1].to_string())
                .or_insert_with(|| js[open..end].to_string());
        }
    }
    out
}

/// Objet donné en valeur (`{…}` ou variable), lu en clés → valeurs simples.
fn resolve_object(raw: &str, vars: &HashMap<String, String>) -> Map<String, Value> {
    let text = if raw.trim_start().starts_with('{') {
        raw.to_string()
    } else {
        match vars.get(raw.trim()) {
            Some(t) => t.clone(),
            None => return Map::new(),
        }
    };
    let mut out = Map::new();
    for (k, v) in object_entries(&text) {
        if let Some(s) = scalar(&v) {
            out.insert(k, s);
        } else if v.trim_start().starts_with('{') || vars.contains_key(v.trim()) {
            out.insert(k, Value::Object(resolve_object(&v, vars)));
        }
    }
    out
}

/// Première classe `framer-…` d'un nom de classe.
fn main_class(cls: &str) -> Option<String> {
    cls.split_whitespace()
        .find(|c| c.starts_with("framer-") && !c.starts_with("framer-v-"))
        .map(str::to_string)
}

/// Propriétés d'un appel d'élément commençant à `pos` jusqu'à ses enfants (`children:`) : les
/// clés de Framer sont triées, les enfants viennent après la classe.
fn props_until_children(js: &str, from: usize, max: usize) -> &str {
    let end = (from + max).min(js.len());
    let mut end = end;
    while !js.is_char_boundary(end) {
        end -= 1;
    }
    let slice = &js[from..end];
    match slice.find("children:") {
        Some(i) => &slice[..i],
        None => slice,
    }
}

// -------------------------------------------------------------------------------------------------
// Effets lus dans les modules

/// Effet d'apparition au défilement d'un élément.
#[derive(Debug, Clone, PartialEq)]
pub struct Appear {
    /// État de départ (`opacity`, `x`, `y`, `scale`, `rotate`).
    pub enter: Map<String, Value>,
    /// Transition (`type`, `duration`, `delay`, `bounce`).
    pub transition: Map<String, Value>,
    /// Joué une fois (`__framer__animateOnce`).
    pub once: bool,
}

/// Ce que les modules du site disent de chaque élément.
#[derive(Debug, Default)]
pub struct Modules {
    /// Classe de l'élément → effet d'apparition.
    pub appear: HashMap<String, Appear>,
    /// Règles `:hover` (états de survol des composants).
    pub hover_css: String,
    /// Code d'une barre de progression liée au défilement de son conteneur (`ScrollProgress`).
    pub scroll_progress: bool,
    /// Défilement doux (composant Lenis).
    pub smooth_scroll: bool,
    /// Composants dont une variante s'affiche au clic (contenu absent de la page publiée).
    pub click_components: Vec<String>,
    /// Leur code : rendu de la variante ouverte relu (D-111).
    pub clicks: Vec<ClickComponent>,
    /// Adresses des données du CMS (`.framercms`) et module de la collection.
    pub cms: Vec<CmsRef>,
    /// Langues du site (`{code:`fr-FR`,id:`K_Klh4yeb`,slug:`fr`}`), la principale d'abord.
    pub locales: Vec<FramerLocale>,
    /// Pages des contenus : (adresse avec le paramètre, identifiant du champ « slug »).
    pub routes: Vec<(String, String)>,
    /// Composants dont la variante suit la section affichée (navigation « À propos / Projets »).
    pub spy: Vec<Spy>,
    /// Classe d'un composant (`framer-fznOQ`) → son nom dans Framer (`displayName`).
    pub component_names: HashMap<String, String>,
    /// Lu module par module, rassemblé dans `spy` à la fin du chargement.
    spy_targets: Vec<Vec<String>>,
    variant_class: HashMap<String, String>,
    variant_rules: Vec<(String, String, Vec<String>)>,
    variant_duration: HashMap<String, f64>,
}

/// Composant à deux variantes passant de l'une à l'autre au clic (carte qui se déplie).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClickComponent {
    pub name: String,
    /// Variante par défaut (celle de la page publiée) et variante ouverte au clic.
    pub default: String,
    pub open: String,
    /// Toutes les variantes du composant.
    pub variants: Vec<String>,
    /// Code du composant (définitions et rendu).
    pub source: String,
}

/// Composant dont la variante change avec la section affichée (`__framer__targets` :
/// première variante pour la première section, etc.).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spy {
    /// Classe de la variante par défaut, portée par l'élément racine dans la page publiée.
    pub root: String,
    /// Variantes suivantes, dans l'ordre des sections : (classe de l'élément, déclarations).
    pub steps: Vec<Vec<(String, Vec<String>)>>,
    /// Durée de la transition entre variantes (s).
    pub duration: f64,
}

/// Langue d'un site Framer.
#[derive(Debug, Clone, PartialEq)]
pub struct FramerLocale {
    pub code: String,
    pub id: String,
    pub name: String,
    /// Préfixe des adresses (`fr`) ; vide pour la langue principale.
    pub slug: String,
}

/// Collection du CMS déclarée dans un module : schéma et fichiers par langue.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CmsRef {
    /// Adresse du module de la collection (sur framerusercontent.com).
    pub module_url: String,
    /// Identifiant de langue de Framer (`default`, `K_Klh4yeb`) → fichiers de données.
    pub chunks: Vec<(String, String)>,
    /// Champs : identifiant → (titre, type de Framer).
    pub fields: Vec<(String, String, String)>,
}

impl Modules {
    /// Lit les modules copiés dans `root/_externe/framerusercontent.com`.
    pub fn load(root: &Path) -> Self {
        let mut files = Vec::new();
        collect_mjs(&root.join("_externe/framerusercontent.com"), &mut files);
        files.sort();
        let mut m = Modules::default();
        for f in &files {
            let Ok(js) = std::fs::read_to_string(f) else {
                continue;
            };
            m.read(&js, f.file_name().and_then(|n| n.to_str()).unwrap_or(""));
        }
        m.finish();
        m
    }

    /// Classe de variante (`framer-v-…`) d'un identifiant de variante.
    pub fn class_of_variant(&self, id: &str) -> Option<&str> {
        self.variant_class.get(id).map(String::as_str)
    }

    /// Styles d'une variante, élément par élément : (classe de l'élément, déclarations).
    pub fn variant_styles<'a>(
        &'a self,
        id: &'a str,
    ) -> impl Iterator<Item = (&'a str, &'a [String])> {
        self.variant_rules
            .iter()
            .filter(move |(v, _, _)| v == id)
            .map(|(_, el, d)| (el.as_str(), d.as_slice()))
    }

    /// Durée de transition des variantes d'un composant (s).
    pub fn variant_duration(&self, id: &str) -> f64 {
        self.variant_duration.get(id).copied().unwrap_or(0.3)
    }

    /// Rassemble les composants qui suivent la section affichée (variantes lues dans d'autres
    /// modules que leur utilisation).
    pub fn finish(&mut self) {
        for targets in std::mem::take(&mut self.spy_targets) {
            let Some(root) = targets.first().and_then(|t| self.variant_class.get(t)) else {
                continue;
            };
            let steps = targets[1..]
                .iter()
                .map(|t| {
                    self.variant_rules
                        .iter()
                        .filter(|(v, _, _)| v == t)
                        .map(|(_, el, d)| (el.clone(), d.clone()))
                        .collect()
                })
                .collect();
            let spy = Spy {
                root: root.clone(),
                steps,
                duration: targets
                    .get(1)
                    .and_then(|t| self.variant_duration.get(t))
                    .copied()
                    .unwrap_or(0.3),
            };
            if !self.spy.iter().any(|s| s.root == spy.root) {
                self.spy.push(spy);
            }
        }
    }

    /// Ajoute ce que dit un module (`name` : nom du fichier).
    pub fn read(&mut self, js: &str, name: &str) {
        let vars = object_vars(js);
        self.read_appear(js, &vars);
        self.read_hover(js, &vars);
        self.read_cms(js);
        let loc = regex::Regex::new(r"\{code:`([^`]*)`,id:`([^`]*)`,name:`([^`]*)`,slug:`([^`]*)`")
            .expect("regex");
        for c in loc.captures_iter(js) {
            if !self.locales.iter().any(|l| l.id == c[2]) {
                self.locales.push(FramerLocale {
                    code: c[1].to_string(),
                    id: c[2].to_string(),
                    name: c[3].to_string(),
                    slug: c[4].to_string(),
                });
            }
        }
        let route = regex::Regex::new(r"path:`(/[^`]*):([A-Za-z0-9_]{9})`").expect("regex");
        for c in route.captures_iter(js) {
            let r = (c[1].to_string(), c[2].to_string());
            if !self.routes.contains(&r) {
                self.routes.push(r);
            }
        }
        if name.to_ascii_lowercase().starts_with("lenis")
            || js.contains("new Lenis(")
            || js.contains("lenis-smooth")
        {
            self.smooth_scroll = true;
        }
        // Composants à variantes choisies au clic : table des noms de variantes
        // (`{Default:`id`,Expanded:`id`}`) suivie, avant le nom du composant, d'un appui.
        let id_re = regex::Regex::new(r"`([A-Za-z0-9_]{9})`").expect("regex");
        let default_re = regex::Regex::new(r"defaultVariant:`([A-Za-z0-9_]{9})`").expect("regex");
        let open_re = regex::Regex::new(r"\(\)=>[\w$]+\(`([A-Za-z0-9_]{9})`\)").expect("regex");
        let map_re = regex::Regex::new(r"\{(?:[A-Z][\w ]*|`[^`]+`):`[A-Za-z0-9_]{9}`(?:,(?:[A-Z][\w ]*|`[^`]+`):`[A-Za-z0-9_]{9}`)+\}").expect("regex");
        for m in map_re.find_iter(js) {
            let rest = &js[m.end()..];
            let Some(d) = rest.find("displayName=`") else {
                continue;
            };
            if !rest[..d].contains("onTap") {
                continue;
            }
            let after = &rest[d + 13..];
            let comp = after[..after.find('`').unwrap_or(0)].to_string();
            if !comp.is_empty() && !self.click_components.contains(&comp) {
                self.click_components.push(comp.clone());
                // Code du composant : depuis le début de son bloc de définitions.
                let start = js[..m.start()]
                    .rfind("=t((()=>{")
                    .unwrap_or(m.start().saturating_sub(6000));
                let source = js[start..m.end() + d].to_string();
                let variants: Vec<String> = id_re
                    .captures_iter(m.as_str())
                    .map(|c| c[1].to_string())
                    .collect();
                let default = default_re
                    .captures_iter(&source)
                    .last()
                    .map(|c| c[1].to_string());
                // Variante ouverte : choisie par un appui (`O(`id`)`), autre que celle par défaut.
                let open = open_re
                    .captures_iter(&source)
                    .map(|c| c[1].to_string())
                    .find(|v| Some(v) != default.as_ref() && variants.contains(v));
                if let (Some(default), Some(open)) = (default, open)
                    && variants.len() == 2
                {
                    self.clicks.push(ClickComponent {
                        name: comp,
                        default,
                        open,
                        variants,
                        source,
                    });
                }
            }
        }
        // Noms des composants : `X.displayName=`Nom``, après la classe du composant (`=`framer-AbCdE``).
        let scope_re = regex::Regex::new(r"=`(framer-[A-Za-z0-9]{5})`").expect("regex");
        let name_re = regex::Regex::new(r"\.displayName=`([^`]+)`").expect("regex");
        let mut from = 0;
        for c in name_re.captures_iter(js) {
            let at = c.get(0).expect("m").start();
            if let Some(scope) = scope_re
                .captures_iter(&js[from..at])
                .map(|x| x[1].to_string())
                .filter(|s| s[7..].chars().any(|c| c.is_ascii_uppercase()))
                .last()
            {
                self.component_names
                    .entry(scope)
                    .or_insert_with(|| c[1].to_string());
            }
            from = c.get(0).expect("m").end();
        }
        // Variante qui suit la section affichée : `__framer__targets:[{…target:`id`},…]`.
        let targets_re = regex::Regex::new(r"__framer__targets:\[([^\]]*)\]").expect("regex");
        let target_re = regex::Regex::new(r"target:`([A-Za-z0-9_]+)`").expect("regex");
        for c in targets_re.captures_iter(js) {
            let at = c.get(0).expect("m").start();
            let near = &js[at..(at + 600).min(js.len())];
            if !near.contains("__framer__variantAppearEffectEnabled:!0") {
                continue;
            }
            let t: Vec<String> = target_re
                .captures_iter(&c[1])
                .map(|x| x[1].to_string())
                .collect();
            if t.len() > 1 && !self.spy_targets.contains(&t) {
                self.spy_targets.push(t);
            }
        }
        // Barre de progression liée au défilement de son conteneur (composant de code).
        if js.contains("overflowY") && js.contains("scrollHeight") && js.contains("clientHeight") {
            self.scroll_progress = true;
        }
    }

    fn read_appear(&mut self, js: &str, vars: &HashMap<String, String>) {
        for (pos, _) in js.match_indices("__framer__animate:") {
            let props = props_until_children(js, pos, 3000);
            if !props.contains("__framer__styleAppearEffectEnabled:!0") {
                continue;
            }
            let get = |key: &str| -> Option<String> {
                let i = props.find(&format!("{key}:"))? + key.len() + 1;
                let rest = &props[i..];
                let end = if rest.starts_with(['{', '`', '"', '[']) {
                    literal_end(rest, 0)?
                } else {
                    rest.find([',', '}']).unwrap_or(rest.len())
                };
                Some(rest[..end].to_string())
            };
            let Some(cls) = get("className")
                .and_then(|c| scalar(&c))
                .and_then(|c| c.as_str().and_then(main_class))
            else {
                continue;
            };
            let enter = get("__framer__enter")
                .map(|e| resolve_object(&e, vars))
                .unwrap_or_default();
            let transition = get("__framer__animate")
                .map(|a| resolve_object(&a, vars))
                .and_then(|a| a.get("transition").and_then(Value::as_object).cloned())
                .unwrap_or_default();
            let once = get("__framer__animateOnce").is_none_or(|v| v.trim() != "!1");
            self.appear.entry(cls).or_insert(Appear {
                enter,
                transition,
                once,
            });
        }
    }

    fn read_hover(&mut self, js: &str, vars: &HashMap<String, String>) {
        // Classes des variantes : `{id:`framer-v-…`,…}`.
        let mut variant_class: HashMap<String, String> = HashMap::new();
        for obj in vars.values() {
            let entries = object_entries(obj);
            if !entries.is_empty()
                && entries.iter().all(|(_, v)| {
                    scalar(v)
                        .and_then(|s| s.as_str().map(|s| s.starts_with("framer-v-")))
                        .unwrap_or(false)
                })
            {
                for (k, v) in entries {
                    if let Some(Value::String(c)) = scalar(&v) {
                        variant_class.insert(k, c);
                    }
                }
            }
        }
        if variant_class.is_empty() {
            return;
        }
        // Transition du composant (première transition à ressort ou à durée du module).
        let transition = vars
            .values()
            .map(|o| resolve_object(o, vars))
            .find(|o| o.contains_key("duration") && o.contains_key("type"))
            .unwrap_or_default();
        let dur = transition
            .get("duration")
            .and_then(Value::as_f64)
            .unwrap_or(0.3);
        let re = regex::Regex::new(r#""([A-Za-z0-9_]{6,12})-hover":\{"#).expect("regex");
        let class_re = regex::Regex::new(r"className:`([^`$]+)`").expect("regex");
        for (k, v) in &variant_class {
            self.variant_class.insert(k.clone(), v.clone());
            self.variant_duration.insert(k.clone(), dur);
        }
        // Styles des autres variantes, élément par élément : `variants:{id:{…}}`.
        for (pos, _) in js.match_indices("variants:{") {
            let open = pos + "variants:".len();
            let Some(end) = literal_end(js, open) else {
                continue;
            };
            let before = &js[open.saturating_sub(4000)..open];
            let Some(el) = class_re
                .captures_iter(before)
                .last()
                .and_then(|m| main_class(&m[1]))
            else {
                continue;
            };
            for (vid, obj) in object_entries(&js[open..end]) {
                if !variant_class.contains_key(&vid) || !obj.starts_with('{') {
                    continue;
                }
                let decls: Vec<String> = resolve_object(&obj, vars)
                    .iter()
                    .filter_map(|(k, v)| css_decl(k, v))
                    .collect();
                if !decls.is_empty() {
                    self.variant_rules.push((vid, el.to_string(), decls));
                }
            }
        }
        let mut rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for c in re.captures_iter(js) {
            let Some(vclass) = variant_class.get(&c[1]) else {
                continue;
            };
            let open = c.get(0).expect("m").end() - 1;
            let Some(end) = literal_end(js, open) else {
                continue;
            };
            // Élément : dernière classe écrite avant ses variantes.
            let before = &js[open.saturating_sub(4000)..open];
            let Some(el) = class_re
                .captures_iter(before)
                .last()
                .and_then(|m| main_class(&m[1]))
            else {
                continue;
            };
            let style = resolve_object(&js[open..end], vars);
            let decls: Vec<String> = style.iter().filter_map(|(k, v)| css_decl(k, v)).collect();
            if decls.is_empty() {
                continue;
            }
            let props: Vec<String> = style
                .keys()
                .filter_map(|k| css_prop(k))
                .map(|p| format!("{p} {dur}s cubic-bezier(.33,1,.68,1)"))
                .collect();
            rules
                .entry(format!(".{vclass} .{el}"))
                .or_default()
                .push(format!("transition:{}", props.join(",")));
            rules
                .entry(format!(".{vclass}:hover .{el}"))
                .or_default()
                .extend(decls.into_iter().map(|d| format!("{d}!important")));
        }
        for (sel, decls) in rules {
            self.hover_css
                .push_str(&format!("{sel}{{{}}}\n", decls.join(";")));
        }
    }

    fn read_cms(&mut self, js: &str) {
        // `chunks:[new URL(`./X-chunk-default-0.framercms`,`https://…/modules/…/X.js`).href
        //  .replace(`/modules/`,`/cms/`)]`, dans `collectionByLocaleId:{default:…,K_Klh4yeb:…}`.
        let re = regex::Regex::new(
            r#"new URL\(`\./([^`]+-chunk-([A-Za-z0-9_]+)-\d+\.framercms)`,`(https://[^`]+)`\)\.href(\.replace\(`/modules/`,`/cms/`\))?"#,
        )
        .expect("regex");
        let mut by_module: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for c in re.captures_iter(js) {
            let Ok(base) = url::Url::parse(&c[3]) else {
                continue;
            };
            let Ok(mut u) = base.join(&c[1]) else {
                continue;
            };
            if c.get(4).is_some() {
                let p = u.path().replacen("/modules/", "/cms/", 1);
                u.set_path(&p);
            }
            let e = by_module.entry(c[3].to_string()).or_default();
            let item = (c[2].to_string(), u.to_string());
            if !e.contains(&item) {
                e.push(item);
            }
        }
        if by_module.is_empty() {
            return;
        }
        let fields = cms_fields(js);
        for (module_url, chunks) in by_module {
            if self.cms.iter().any(|c| c.module_url == module_url) {
                continue;
            }
            self.cms.push(CmsRef {
                module_url,
                chunks,
                fields: fields.clone(),
            });
        }
    }
}

/// Champs d'une collection écrits dans son module : `id:{…,title:"Titre",type:l.String}`.
pub fn cms_fields(js: &str) -> Vec<(String, String, String)> {
    let re = regex::Regex::new(r"([A-Za-z0-9_]{9}):\{").expect("regex");
    let mut fields: Vec<(String, String, String)> = Vec::new();
    let mut after = 0;
    for c in re.captures_iter(js) {
        let m = c.get(0).expect("m");
        if m.start() < after {
            continue;
        }
        let open = m.end() - 1;
        let Some(end) = literal_end(js, open) else {
            continue;
        };
        let entries = object_entries(&js[open..end]);
        let get = |k: &str| {
            entries
                .iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.clone())
        };
        let (Some(title), Some(ty)) = (get("title").and_then(|t| scalar(&t)), get("type")) else {
            continue;
        };
        let Some(title) = title.as_str() else {
            continue;
        };
        let ty = ty
            .trim_matches(['`', '"'])
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_string();
        after = end;
        if !fields.iter().any(|(id, _, _)| id == &c[1]) {
            fields.push((c[1].to_string(), title.to_string(), ty));
        }
    }
    fields
}

fn collect_mjs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_mjs(&p, out);
        } else if p.extension().is_some_and(|x| x == "mjs" || x == "js") {
            out.push(p);
        }
    }
}

/// Propriété CSS d'une clé de style de Framer (`WebkitFilter` → `-webkit-filter`).
fn css_prop(k: &str) -> Option<String> {
    match k {
        "x" | "y" | "scale" | "rotate" | "transition" => return Some("transform".into()),
        _ => {}
    }
    if !k.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    let mut out = String::new();
    for c in k.chars() {
        if c.is_ascii_uppercase() {
            // `WebkitBackdropFilter` → `-webkit-backdrop-filter`.
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Déclaration CSS d'une clé de style de Framer ; transformations regroupées ailleurs.
pub(crate) fn css_decl(k: &str, v: &Value) -> Option<String> {
    let val = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            let f = n.as_f64()?;
            match k {
                "x" | "y" => return Some(format!("translate:{}", px(k, f))),
                "scale" => return Some(format!("scale:{f}")),
                "rotate" => return Some(format!("rotate:{f}deg")),
                "opacity" | "zIndex" | "flexGrow" => f.to_string(),
                _ => format!("{f}px"),
            }
        }
        _ => return None,
    };
    if k == "transition" {
        return None;
    }
    Some(format!("{}:{val}", css_prop(k)?))
}

fn px(k: &str, f: f64) -> String {
    if k == "x" {
        format!("{f}px 0")
    } else {
        format!("0 {f}px")
    }
}

/// Règles de la feuille de style de Framer pour l'état survolé (`.framer-v-x.hover .y`) en
/// règles `:hover`.
pub fn hover_classes_to_pseudo(css: &str) -> String {
    let re = regex::Regex::new(r"(\.framer-v-[A-Za-z0-9_-]+)\.hover\b").expect("regex");
    re.replace_all(css, "$1:hover").into_owned()
}

// -------------------------------------------------------------------------------------------------
// Données du CMS (`.framercms`)

/// Valeur décodée d'un fichier `.framercms`.
#[derive(Debug, Clone, PartialEq)]
pub enum CmsValue {
    Null,
    String(String),
    /// Image : objet JSON (`src`, `srcSet`, `pixelWidth`…).
    Image(String),
    /// Texte riche : arbre JSON de Framer.
    Rich(String),
    /// Lien (adresse entre guillemets JSON).
    Link(String),
    Date(u64),
    Array(Vec<CmsValue>),
    Object(Vec<(String, CmsValue)>),
}

/// Contenus d'un fichier de données `…-chunk-…framercms` : champs par identifiant.
pub fn decode_chunk(d: &[u8]) -> Result<Vec<Vec<(String, CmsValue)>>, String> {
    let mut r = Reader { d, i: 0 };
    let n = r.u32()?;
    let mut items = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let nf = r.u16()?;
        let mut fields = Vec::with_capacity(nf as usize);
        for _ in 0..nf {
            let k = r.string()?;
            let v = r.value()?;
            fields.push((k, v));
        }
        items.push(fields);
    }
    Ok(items)
}

struct Reader<'a> {
    d: &'a [u8],
    i: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let s = self
            .d
            .get(self.i..self.i + n)
            .ok_or_else(|| format!("fin de fichier à {}", self.i))?;
        self.i += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn string(&mut self) -> Result<String, String> {
        let n = self.u32()? as usize;
        let at = self.i;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| format!("texte illisible à {at}"))
    }
    fn value(&mut self) -> Result<CmsValue, String> {
        let at = self.i;
        Ok(match self.u8()? {
            0x00 => CmsValue::Null,
            0x01 => {
                let n = self.u16()?;
                let mut a = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    a.push(self.value()?);
                }
                CmsValue::Array(a)
            }
            0x04 => {
                let b = self.take(8)?;
                CmsValue::Date(u64::from_be_bytes(b.try_into().expect("8 octets")))
            }
            0x07 => CmsValue::Link(self.string()?),
            0x09 => {
                let n = self.u16()?;
                let mut o = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    let k = self.string()?;
                    o.push((k, self.value()?));
                }
                CmsValue::Object(o)
            }
            0x0a => CmsValue::Image(self.string()?),
            0x0b => {
                let _format = self.u8()?;
                CmsValue::Rich(self.string()?)
            }
            0x0c => CmsValue::String(self.string()?),
            t => return Err(format!("type {t:#04x} inconnu à {at}")),
        })
    }
}

/// Texte riche de Framer (`[1,[4,"p",{…},[5,"texte"]],…]`) en HTML.
pub fn rich_to_html(json_text: &str) -> String {
    let Ok(v) = serde_json::from_str::<Value>(json_text) else {
        return String::new();
    };
    let mut out = String::new();
    if let Some(a) = v.as_array() {
        for n in a.iter().skip(1) {
            rich_node(n, &mut out);
        }
    }
    out
}

fn rich_node(n: &Value, out: &mut String) {
    let Some(a) = n.as_array() else {
        if let Some(s) = n.as_str() {
            out.push_str(&escape(s));
        }
        return;
    };
    match a.first().and_then(Value::as_u64) {
        // Élément : [4, balise, attributs, enfants…].
        Some(4) => {
            let tag = a.get(1).and_then(Value::as_str).unwrap_or("span");
            let tag: String = tag.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            out.push('<');
            out.push_str(&tag);
            if let Some(attrs) = a.get(2).and_then(Value::as_object) {
                for (k, v) in attrs {
                    if k == "dir" || !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                        continue;
                    }
                    if let Some(s) = v.as_str() {
                        out.push_str(&format!(" {k}=\"{}\"", escape(s)));
                    }
                }
            }
            let void = matches!(tag.as_str(), "br" | "img" | "hr");
            out.push('>');
            for c in a.iter().skip(3) {
                rich_node(c, out);
            }
            if !void {
                out.push_str(&format!("</{tag}>"));
            }
        }
        // Texte : [5, "…"].
        Some(5) => {
            for c in a.iter().skip(1) {
                if let Some(s) = c.as_str() {
                    out.push_str(&escape(s));
                }
            }
        }
        _ => {
            for c in a.iter().skip(1) {
                rich_node(c, out);
            }
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Adresse d'une image du CMS (objet `{"src":…}`), sans les paramètres de taille.
pub fn image_src(json_text: &str) -> Option<String> {
    let v: Value = serde_json::from_str(json_text).ok()?;
    let src = v.get("src")?.as_str()?;
    let mut u = url::Url::parse(src).ok()?;
    u.set_query(None);
    Some(u.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD: &str = r#"R={DVs_1soLj:{hover:!0},UdgeVogri:{hover:!0}},z=[`UdgeVogri`,`DVs_1soLj`],B=`framer-GEW64`,V={DVs_1soLj:`framer-v-11pekp5`,lD6oBgyg0:`framer-v-dgwa72`,UdgeVogri:`framer-v-1ydxgvo`},H={bounce:0,delay:0,duration:1.1,type:`spring`};p(m.div,{className:`framer-1gnamzt`,"data-framer-name":`Content`,layoutId:`Ik46Nj0mm`,style:{filter:`grayscale(1)`,WebkitFilter:`grayscale(1)`},variants:{"UdgeVogri-hover":{filter:`grayscale(0)`,WebkitFilter:`grayscale(0)`},DVs_1soLj:{filter:`none`}},children:x})"#;

    const PAGE: &str = r#"U={opacity:0,rotate:0,rotateX:0,rotateY:0,scale:1,skewX:0,skewY:0,x:0,y:0},W={bounce:.1,delay:.1,duration:1.5,type:`spring`};o(R,{__framer__animate:{transition:W},__framer__animateOnce:!0,__framer__enter:U,__framer__exit:G,__framer__styleAppearEffectEnabled:!0,__framer__threshold:0,className:`framer-dhvub1`,"data-framer-name":`Intro`,children:[i(`div`,{className:`framer-s8w4t`})]})"#;

    #[test]
    fn litteraux() {
        let e = object_entries(r#"{a:1,"b-c":`x, y`,d:{e:2},f}"#);
        assert_eq!(
            e,
            vec![
                ("a".into(), "1".into()),
                ("b-c".into(), "`x, y`".into()),
                ("d".into(), "{e:2}".into()),
                ("f".into(), "f".into()),
            ]
        );
        assert_eq!(scalar(".1"), Some(json!(0.1)));
        assert_eq!(scalar("!0"), Some(json!(true)));
        assert_eq!(scalar("`a`"), Some(json!("a")));
        assert_eq!(scalar("`a${b}`"), None);
    }

    #[test]
    fn apparition_au_defilement() {
        let mut m = Modules::default();
        m.read(PAGE, "page.mjs");
        let a = &m.appear["framer-dhvub1"];
        assert_eq!(a.enter["opacity"], json!(0.0));
        assert_eq!(a.transition["duration"], json!(1.5));
        assert_eq!(a.transition["delay"], json!(0.1));
        assert!(a.once);
        assert!(!m.appear.contains_key("framer-s8w4t"));
    }

    #[test]
    fn survol_des_variantes() {
        let mut m = Modules::default();
        m.read(CARD, "card.mjs");
        assert!(
            m.hover_css.contains(
                ".framer-v-1ydxgvo:hover .framer-1gnamzt{filter:grayscale(0)!important;-webkit-filter:grayscale(0)!important}"
            ),
            "{}",
            m.hover_css
        );
        assert!(
            m.hover_css
                .contains(".framer-v-1ydxgvo .framer-1gnamzt{transition:filter 1.1s"),
            "{}",
            m.hover_css
        );
        assert_eq!(
            hover_classes_to_pseudo(
                ".framer-jk4NJ.framer-v-1bn6r0u.hover .framer-1rhqd5c{padding:0}"
            ),
            ".framer-jk4NJ.framer-v-1bn6r0u:hover .framer-1rhqd5c{padding:0}"
        );
    }

    #[test]
    fn composant_de_code_et_cms() {
        let js = r#"function P(){let s=e.parentElement;getComputedStyle(s).overflowY;s.scrollHeight-s.clientHeight}ji={Default:`t9ftF54_6`,Expanded:`CZid0QA1b`},Mi=1;function Fi(){let t=onTapabc(()=>setVariant(`CZid0QA1b`))}Fi.displayName=`AI`;$={collectionByLocaleId:{default:new ir({chunks:[new URL(`./ySRpxX3SI-chunk-default-0.framercms`,`https://framerusercontent.com/modules/90tD/CRqe/ySRpxX3SI.js`).href.replace(`/modules/`,`/cms/`)]})}};"#;
        let mut m = Modules::default();
        m.read(js, "page.mjs");
        assert!(m.scroll_progress);
        assert_eq!(m.click_components, vec!["AI".to_string()]);
        assert_eq!(m.cms.len(), 1);
        assert_eq!(
            m.cms[0].chunks,
            vec![(
                "default".to_string(),
                "https://framerusercontent.com/cms/90tD/CRqe/ySRpxX3SI-chunk-default-0.framercms"
                    .to_string()
            )]
        );
        let schema = r#"MtFEKlxCL:{defaultValue:"",title:"Project title",type:l.String},HSiStKSIy:{__vekterDefault:[],control:{controls:{D_NSHH6CX:{title:"Image",type:"responsiveimage"}},type:"object"},title:"Gallery",type:l.Array}"#;
        let fields = cms_fields(schema);
        let titles: Vec<&str> = fields.iter().map(|f| f.1.as_str()).collect();
        assert_eq!(titles, vec!["Project title", "Gallery"]);
    }

    #[test]
    fn donnees_du_cms() {
        // 1 contenu, 3 champs : texte, image (objet dans une liste), texte riche.
        let mut d = vec![0, 0, 0, 1, 0, 3];
        let s = |d: &mut Vec<u8>, t: &str| {
            d.extend((t.len() as u32).to_be_bytes());
            d.extend(t.as_bytes());
        };
        s(&mut d, "title");
        d.push(0x0c);
        s(&mut d, "Cartier");
        s(&mut d, "gallery");
        d.extend([0x01, 0, 1, 0x09, 0, 1]);
        s(&mut d, "img");
        d.push(0x0a);
        s(
            &mut d,
            r#"{"src":"https://framerusercontent.com/images/a.png?width=10"}"#,
        );
        s(&mut d, "body");
        d.extend([0x0b, 1]);
        s(
            &mut d,
            r#"[1,[4,"p",{"dir":"auto"},[5,"Film"]],[4,"h3",{},[5,"A & B"]]]"#,
        );
        let items = decode_chunk(&d).unwrap();
        assert_eq!(
            items[0][0],
            ("title".into(), CmsValue::String("Cartier".into()))
        );
        let CmsValue::Array(g) = &items[0][1].1 else {
            panic!()
        };
        let CmsValue::Object(o) = &g[0] else { panic!() };
        let CmsValue::Image(j) = &o[0].1 else {
            panic!()
        };
        assert_eq!(
            image_src(j).as_deref(),
            Some("https://framerusercontent.com/images/a.png")
        );
        let CmsValue::Rich(r) = &items[0][2].1 else {
            panic!()
        };
        assert_eq!(rich_to_html(r), "<p>Film</p><h3>A &amp; B</h3>");
        assert!(decode_chunk(&[0, 0, 0, 1, 0, 1, 0, 0, 0, 1, b'a', 0x42]).is_err());
    }
}
