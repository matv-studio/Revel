//! Champs de la fiche d'un contenu (étape 3.3). Le contrôle de chaque champ vient du schéma de
//! Pages CMS (`.pages.yml`) : texte, texte long, code, texte riche, image, date (avec heure et
//! format d'écriture), case à cocher, nombre, liste, choix, référence vers une autre
//! collection, objet (sous-fiche). Sans schéma, il est déduit de la valeur. Une valeur refusée
//! (champ obligatoire vide, motif non respecté) n'est pas écrite et le champ le signale.

use crate::richtext::{Format, RichText, looks_like_html};
use crate::site::capitalize;
use crate::{Ui, icons};
use leptos::ev;
use leptos::prelude::*;
use revel_protocol::{CmsCollection, CmsField};
use serde_json::Value;
use wasm_bindgen::JsCast;

/// Collections du site (choix des références).
pub type Cols = Signal<Option<Vec<CmsCollection>>>;

/// Libellé d'un champ : celui du schéma, sinon le nom mis en forme (`date_debut` -> « Date debut »).
pub fn label_of(f: &CmsField) -> String {
    f.label
        .clone()
        .unwrap_or_else(|| capitalize(&f.name.replace(['_', '-'], " ")))
}

/// Type de contrôle d'un champ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Text,
    LongText,
    Code,
    Rich,
    Image,
    Date,
    Bool,
    Number,
    /// Liste de textes (pastilles).
    List,
    Select {
        multiple: bool,
    },
    Reference {
        multiple: bool,
    },
    Object,
    /// Liste d'objets (groupe de champs répété) : une carte par élément.
    ObjectList,
    /// Blocs de Pages CMS : liste d'éléments dont le type (`blockKey`) choisit les champs.
    Blocks,
    /// Valeur structurée sans contrôle dédié : texte JSON.
    Json,
}

fn multiple(f: &CmsField) -> bool {
    match f.options.get("multiple") {
        Some(Value::Bool(b)) => *b,
        Some(Value::Object(_)) => true,
        _ => false,
    }
}

fn all_strings(v: Option<&Value>) -> bool {
    v.is_none_or(|v| v.is_null() || v.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
}

/// Contrôle d'un champ : d'après son type, sinon d'après sa valeur.
pub fn kind_of(f: &CmsField, v: Option<&Value>) -> Kind {
    let Some(t) = f.kind.as_deref() else {
        return infer(v);
    };
    let many = f.list || (matches!(t, "image" | "file") && multiple(f));
    match t {
        "select" => Kind::Select {
            multiple: multiple(f),
        },
        "reference" => Kind::Reference {
            multiple: multiple(f),
        },
        "string" | "text" | "image" | "file" | "list" if many || t == "list" => {
            if all_strings(v) {
                Kind::List
            } else {
                Kind::Json
            }
        }
        "object"
            if many && !f.fields.is_empty() && v.is_none_or(|v| v.is_array() || v.is_null()) =>
        {
            Kind::ObjectList
        }
        "block"
            if !f.fields.is_empty()
                && v.is_none_or(|v| v.is_array() || v.is_object() || v.is_null()) =>
        {
            Kind::Blocks
        }
        _ if many => Kind::Json,
        "string" | "uuid" => Kind::Text,
        "text" => Kind::LongText,
        "code" => Kind::Code,
        "rich_text" => Kind::Rich,
        "image" | "file" => Kind::Image,
        "date" => Kind::Date,
        "boolean" => Kind::Bool,
        "number" => Kind::Number,
        "object" if !f.fields.is_empty() && v.is_none_or(|v| v.is_object() || v.is_null()) => {
            Kind::Object
        }
        "object" | "block" => Kind::Json,
        _ => infer(v),
    }
}

fn infer(v: Option<&Value>) -> Kind {
    match v {
        Some(Value::Bool(_)) => Kind::Bool,
        Some(Value::Number(_)) => Kind::Number,
        Some(Value::Array(a)) if a.iter().all(Value::is_string) => Kind::List,
        Some(Value::Array(_) | Value::Object(_)) => Kind::Json,
        Some(Value::String(s)) if is_date(s) => Kind::Date,
        Some(Value::String(s)) if is_image(s) => Kind::Image,
        Some(Value::String(s)) if looks_like_html(s) => Kind::Rich,
        Some(Value::String(s)) if s.contains('\n') || s.chars().count() > 90 => Kind::LongText,
        _ => Kind::Text,
    }
}

/// Texte commençant par une date ISO (`2024-05-01…`).
pub fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[7] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[8..10].iter().all(u8::is_ascii_digit)
}

fn is_image(s: &str) -> bool {
    let l = s.to_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".avif", ".gif", ".svg"]
        .iter()
        .any(|e| l.ends_with(e))
}

/// Valeur simple en texte (`null` : vide).
pub fn scalar(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        v => v.to_string(),
    }
}

/// Nom de fichier tiré d'un texte (`L'Été à Oka` -> `l-ete-a-oka`).
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.trim().to_lowercase().chars() {
        let c = match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'œ' => {
                out.push('o');
                'e'
            }
            'æ' => {
                out.push('a');
                'e'
            }
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !out.ends_with('-') {
            out.push(c);
        }
    }
    let out = out.trim_matches('-').chars().take(80).collect::<String>();
    if out.is_empty() {
        "contenu".into()
    } else {
        out
    }
}

// ------------------------------------------------------------------ Dates

/// Date et heure, sans fuseau (comme les champs de saisie).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Parts {
    y: i32,
    mo: u32,
    d: u32,
    h: u32,
    mi: u32,
    s: u32,
}

/// Élément d'un format de date-fns (`yyyy-MM-dd'T'HH:mm:ss`, celui de Pages CMS).
enum Tok {
    Num(char, usize),
    Lit(String),
}

fn tokens(fmt: &str) -> Vec<Tok> {
    let cs: Vec<char> = fmt.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '\'' {
            // Texte entre apostrophes ; « '' » : une apostrophe.
            let mut lit = String::new();
            i += 1;
            while i < cs.len() {
                if cs[i] == '\'' {
                    if cs.get(i + 1) == Some(&'\'') {
                        lit.push('\'');
                        i += 2;
                        continue;
                    }
                    break;
                }
                lit.push(cs[i]);
                i += 1;
            }
            i += 1;
            if lit.is_empty() {
                lit.push('\'');
            }
            out.push(Tok::Lit(lit));
        } else if "yMdHms".contains(c) {
            let start = i;
            while i < cs.len() && cs[i] == c {
                i += 1;
            }
            out.push(Tok::Num(c, i - start));
        } else {
            out.push(Tok::Lit(c.to_string()));
            i += 1;
        }
    }
    out
}

fn parse_date(fmt: &str, s: &str) -> Option<Parts> {
    let mut p = Parts {
        y: 1970,
        mo: 1,
        d: 1,
        h: 0,
        mi: 0,
        s: 0,
    };
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    for t in tokens(fmt) {
        match t {
            Tok::Lit(l) => {
                for c in l.chars() {
                    if b.get(i) != Some(&c) {
                        return None;
                    }
                    i += 1;
                }
            }
            Tok::Num(c, n) => {
                let (min, max) = match (c, n) {
                    ('y', 2) => (2, 2),
                    ('y', _) => (4, 4),
                    (_, 1) => (1, 2),
                    (_, n) => (n, n),
                };
                let start = i;
                while i < b.len() && i - start < max && b[i].is_ascii_digit() {
                    i += 1;
                }
                if i - start < min {
                    return None;
                }
                let v: u32 = b[start..i].iter().collect::<String>().parse().ok()?;
                match c {
                    'y' if n == 2 => p.y = 2000 + v as i32,
                    'y' => p.y = v as i32,
                    'M' => p.mo = v,
                    'd' => p.d = v,
                    'H' => p.h = v,
                    'm' => p.mi = v,
                    _ => p.s = v,
                }
            }
        }
    }
    (i == b.len()
        && (1..=12).contains(&p.mo)
        && (1..=31).contains(&p.d)
        && p.h < 24
        && p.mi < 60
        && p.s < 61)
        .then_some(p)
}

fn format_date(fmt: &str, p: Parts) -> String {
    tokens(fmt)
        .into_iter()
        .map(|t| match t {
            Tok::Lit(l) => l,
            Tok::Num(c, n) => {
                let (v, w) = match c {
                    'y' if n == 2 => (p.y.rem_euclid(100) as u32, 2),
                    'y' => (p.y.max(0) as u32, n),
                    'M' => (p.mo, n.min(2)),
                    'd' => (p.d, n.min(2)),
                    'H' => (p.h, n.min(2)),
                    'm' => (p.mi, n.min(2)),
                    _ => (p.s, n.min(2)),
                };
                format!("{v:0w$}")
            }
        })
        .collect()
}

/// Date ISO, avec ou sans heure (`2024-05-01`, `2024-05-01T10:30`, `2024-05-01 10:30:00Z`).
fn iso_parts(v: &str) -> Option<Parts> {
    let v = v.trim();
    let v = if v.as_bytes().get(10) == Some(&b' ') {
        format!("{}T{}", &v[..10], &v[11..])
    } else {
        v.to_string()
    };
    [
        ("yyyy-MM-dd'T'HH:mm:ss", 19),
        ("yyyy-MM-dd'T'HH:mm", 16),
        ("yyyy-MM-dd", 10),
    ]
    .into_iter()
    .find_map(|(fmt, len)| v.get(..len).and_then(|s| parse_date(fmt, s)))
}

fn time_of(f: &CmsField) -> bool {
    f.options.get("time").and_then(Value::as_bool) == Some(true)
}

fn input_format(time: bool) -> &'static str {
    if time {
        "yyyy-MM-dd'T'HH:mm"
    } else {
        "yyyy-MM-dd"
    }
}

/// Format d'écriture d'une date (`options.format` de Pages CMS, sinon celui du champ de saisie).
fn save_format(f: &CmsField) -> String {
    f.options
        .get("format")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| input_format(time_of(f)).into())
}

/// Valeur enregistrée -> valeur du champ de saisie (`date` ou `datetime-local`).
pub fn date_read(f: &CmsField, v: &str) -> String {
    parse_date(&save_format(f), v.trim())
        .or_else(|| iso_parts(v))
        .map(|p| format_date(input_format(time_of(f)), p))
        .unwrap_or_default()
}

/// Valeur du champ de saisie -> valeur enregistrée, au format du schéma.
pub fn date_write(f: &CmsField, input: &str) -> String {
    iso_parts(input)
        .map(|p| format_date(&save_format(f), p))
        .unwrap_or_default()
}

/// Date du jour au format du champ (valeur d'un nouveau contenu, comme Pages CMS).
pub fn date_now(f: &CmsField) -> String {
    let d = js_sys::Date::new_0();
    let p = Parts {
        y: d.get_full_year() as i32,
        mo: d.get_month() + 1,
        d: d.get_date(),
        h: d.get_hours(),
        mi: d.get_minutes(),
        s: d.get_seconds(),
    };
    format_date(&save_format(f), p)
}

// ------------------------------------------------------------------ Choix et références

/// Choix d'un champ `select` : (valeur, libellé).
pub fn choices(f: &CmsField) -> Vec<(String, String)> {
    f.options
        .get("values")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| match x {
                    Value::Object(o) => {
                        let v = scalar(o.get("value").or(o.get("name"))?);
                        let l = o.get("label").map(scalar).unwrap_or_else(|| v.clone());
                        Some((v, l))
                    }
                    Value::Null => None,
                    x => Some((scalar(x), scalar(x))),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn access<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(v, |acc, k| acc.get(k))
}

/// Modèle de Pages CMS (`{fields.title}`, `{name}`, `{path}`) rempli avec un contenu ; un nom
/// sans `fields.` est aussi cherché parmi les champs.
pub fn interpolate(tpl: &str, item: &Value) -> String {
    let mut out = String::new();
    let mut rest = tpl;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let Some(j) = rest[i..].find('}') else {
            out.push_str(&rest[i..]);
            return out;
        };
        let token = rest[i + 1..i + j].trim();
        let v = access(item, token).or_else(|| access(item, &format!("fields.{token}")));
        out.push_str(&v.map(scalar).unwrap_or_default());
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    out
}

/// Contenus proposés par un champ `reference` : (valeur écrite, libellé), triés par libellé.
pub fn reference_choices(cols: &[CmsCollection], f: &CmsField) -> Vec<(String, String)> {
    let Some(name) = f.options.get("collection").and_then(Value::as_str) else {
        return Vec::new();
    };
    let Some(c) = cols
        .iter()
        .find(|c| c.schema.as_deref() == Some(name))
        .or_else(|| cols.iter().find(|c| c.id == name))
    else {
        return Vec::new();
    };
    let vt = f
        .options
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or("{path}");
    let lt = f
        .options
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("{name}");
    let mut out: Vec<(String, String)> = c
        .entries
        .iter()
        .map(|e| {
            let item = serde_json::json!({
                "name": format!("{}.md", e.file),
                "path": format!("{}/{}.md", c.folder, e.file),
                "fields": e.data,
            });
            let v = interpolate(vt, &item);
            let l = interpolate(lt, &item);
            let l = if l.trim().is_empty() { v.clone() } else { l };
            (v, l)
        })
        .filter(|(v, _)| !v.is_empty())
        .collect();
    out.sort_by_key(|(_, l)| l.to_lowercase());
    out
}

/// Nom du fichier d'un nouveau contenu d'après le modèle de la collection
/// (`{fields.slug}.md`, `{year}-{month}-{day}-{primary}.md`) ; `None` s'il est inutilisable
/// (vide, dans un sous-dossier).
pub fn file_from_template(tpl: &str, primary: &str, data: &Value) -> Option<String> {
    let d = js_sys::Date::new_0();
    let pad = |n: u32| format!("{n:02}");
    let t = tpl
        .replace("{year}", &d.get_full_year().to_string())
        .replace("{month}", &pad(d.get_month() + 1))
        .replace("{day}", &pad(d.get_date()))
        .replace("{hour}", &pad(d.get_hours()))
        .replace("{minute}", &pad(d.get_minutes()))
        .replace("{second}", &pad(d.get_seconds()))
        .replace("{primary}", &format!("{{fields.{primary}}}"))
        .replace("{slug}", &format!("{{fields.{primary}}}"));
    let mut out = String::new();
    let mut rest = t.as_str();
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let j = rest[i..].find('}')?;
        let token = rest[i + 1..i + j].trim().trim_start_matches("fields.");
        let v = access(data, token).map(scalar).unwrap_or_default();
        if !v.trim().is_empty() {
            out.push_str(&slugify(&v));
        }
        rest = &rest[i + j + 1..];
    }
    out.push_str(rest);
    let stem = out
        .strip_suffix(".md")
        .or_else(|| out.strip_suffix(".markdown"))
        .unwrap_or(&out)
        .trim_matches('-');
    (!stem.is_empty() && !stem.contains(['/', '\\'])).then(|| stem.to_string())
}

// ------------------------------------------------------------------ Validation

fn is_empty(v: &Option<Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        Some(Value::Array(a)) => a.is_empty(),
        _ => false,
    }
}

/// Motif d'un champ (expression régulière JavaScript, comme Pages CMS) ; motif illisible :
/// accepté.
fn matches_pattern(pattern: &str, s: &str) -> bool {
    let Some(ctor) = js_sys::Reflect::get(&js_sys::global(), &"RegExp".into())
        .ok()
        .and_then(|c| c.dyn_into::<js_sys::Function>().ok())
    else {
        return true;
    };
    match js_sys::Reflect::construct(&ctor, &js_sys::Array::of1(&pattern.into())) {
        Ok(r) => r.unchecked_into::<js_sys::RegExp>().test(s),
        Err(_) => true,
    }
}

/// Raison du refus d'une valeur, sinon `None`.
pub fn problem(ui: Ui, f: &CmsField, v: &Option<Value>) -> Option<String> {
    if f.required && is_empty(v) {
        return Some(ui.now("cms-required", &[]));
    }
    if let (Some(p), Some(Value::String(s))) = (&f.pattern, v)
        && !s.is_empty()
        && !matches_pattern(p, s)
    {
        return Some(ui.now("cms-pattern", &[]));
    }
    None
}

// ------------------------------------------------------------------ Fiche

/// Ligne de la fiche : libellé (aide au survol, marque d'un champ obligatoire), contrôle,
/// message d'erreur. `set` reçoit la nouvelle valeur (`None` : champ retiré) ; un objet affiche
/// ses champs dans un cadre.
pub fn field_row(
    f: CmsField,
    value: Signal<Option<Value>>,
    set: Callback<Option<Value>>,
    cols: Cols,
    aside: Option<AnyView>,
) -> AnyView {
    let ui = expect_context::<Ui>();
    let kind = kind_of(&f, value.get_untracked().as_ref());
    let label = label_of(&f);
    let help = f.description.clone();
    let name = f.name.clone();
    let head = view! {
        <span class="label cms-label" title=help.clone().unwrap_or_else(|| f.name.clone())>
            <span class="cms-label-text">{label}</span>
            {f.required.then(|| view! { <span class="cms-req" title=ui.t("cms-required-mark")>"*"</span> })}
            {help.is_some().then(icons::info)}
        </span>
    };
    if matches!(kind, Kind::ObjectList | Kind::Blocks) {
        return repeat_view(ui, f, kind, value, set, cols, head.into_any());
    }
    if kind == Kind::Object {
        let subs = f.fields.clone();
        return view! {
            <fieldset class="cms-object" data-field=name>
                <legend>{head}</legend>
                {subs.into_iter().filter(|s| !s.hidden).map(|s| {
                    let key = StoredValue::new(s.name.clone());
                    let sub_value = Signal::derive(move || value.get().and_then(|o| o.get(key.get_value()).cloned()));
                    let sub_set = Callback::new(move |v: Option<Value>| {
                        let mut o = value.get_untracked().and_then(|x| x.as_object().cloned()).unwrap_or_default();
                        match v {
                            Some(v) => { o.insert(key.get_value(), v); }
                            None => { o.remove(&key.get_value()); }
                        }
                        set.run(Some(Value::Object(o)));
                    });
                    field_row(s, sub_value, sub_set, cols, None)
                }).collect_view()}
            </fieldset>
        }
        .into_any();
    }
    let error = RwSignal::new(None::<String>);
    let fs = StoredValue::new(f);
    let commit = Callback::new(move |v: Option<Value>| {
        if let Some(m) = fs.with_value(|f| problem(ui, f, &v)) {
            error.set(Some(m));
            return;
        }
        error.set(None);
        set.run(v);
    });
    let control = control(kind, fs, value, commit, cols, error);
    let inline = kind == Kind::Bool;
    let err = move || {
        error
            .get()
            .map(|m| view! { <span class="field-error" role="alert">{m}</span> })
    };
    // Contrôle composé (texte riche, pastilles) : pas dans un `<label>`, qui renverrait chaque
    // clic vers son premier bouton.
    let composite = aside.is_some()
        || matches!(
            kind,
            Kind::Rich
                | Kind::List
                | Kind::Select { multiple: true }
                | Kind::Reference { multiple: true }
        );
    // Actions à droite du libellé (valeur hors schéma : avertissement, ajout au schéma).
    let head = match aside {
        Some(a) => view! { <div class="cms-field-head">{head}{a}</div> }.into_any(),
        None => head.into_any(),
    };
    if composite {
        view! {
            <div class="cms-item-field" class:is-inline=inline class:is-invalid=move || error.with(Option::is_some) data-field=name>
                {head}{control}{err}
            </div>
        }
        .into_any()
    } else {
        view! {
            <label class="cms-item-field" class:is-inline=inline class:is-invalid=move || error.with(Option::is_some) data-field=name>
                {head}{control}{err}
            </label>
        }
        .into_any()
    }
}

fn control(
    kind: Kind,
    f: StoredValue<CmsField>,
    value: Signal<Option<Value>>,
    commit: Callback<Option<Value>>,
    cols: Cols,
    error: RwSignal<Option<String>>,
) -> AnyView {
    let ui = expect_context::<Ui>();
    let text = move || match value.get() {
        Some(Value::String(s)) => s,
        Some(Value::Number(n)) => n.to_string(),
        Some(v @ (Value::Array(_) | Value::Object(_))) => {
            serde_json::to_string_pretty(&v).unwrap_or_default()
        }
        _ => String::new(),
    };
    let commit_text = move |v: String| commit.run(Some(Value::String(v)));
    let strings = move || -> Vec<String> {
        match value.get() {
            Some(Value::Array(a)) => a.iter().map(scalar).filter(|s| !s.is_empty()).collect(),
            Some(Value::String(s)) if !s.is_empty() => vec![s],
            _ => Vec::new(),
        }
    };
    let write_strings = move |v: Vec<String>| {
        commit.run(Some(Value::Array(
            v.into_iter().map(Value::String).collect(),
        )))
    };
    match kind {
        Kind::Bool => view! {
            <span class="switch">
                <input type="checkbox" prop:checked=move || matches!(value.get(), Some(Value::Bool(true)))
                    on:change=move |e| commit.run(Some(Value::Bool(event_target_checked(&e))))/>
                <span class="switch-track"></span>
            </span>
        }
        .into_any(),
        Kind::Number => view! {
            <input class="input" type="number" step="any" prop:value=text
                on:change=move |e| {
                    let v = event_target_value(&e);
                    let n = v.trim().parse::<f64>().ok().and_then(|x| {
                        if x.fract() == 0.0 && x.abs() < 1e15 { Some(serde_json::Number::from(x as i64)) } else { serde_json::Number::from_f64(x) }
                    });
                    commit.run(n.map(Value::Number).or(Some(Value::Null)));
                }/>
        }
        .into_any(),
        Kind::Date => {
            let time = f.with_value(time_of);
            let typed = f.with_value(|f| f.kind.is_some());
            view! {
                <input class="input" type=if time { "datetime-local" } else { "date" }
                    prop:value=move || f.with_value(|f| date_read(f, &text()))
                    on:change=move |e| {
                        let v = event_target_value(&e);
                        let out = if v.is_empty() {
                            v
                        } else if typed {
                            f.with_value(|f| date_write(f, &v))
                        } else {
                            // Sans schéma : l'heure éventuelle de l'ancienne valeur est gardée.
                            let old = text();
                            if old.len() > 10 && is_date(&old) { format!("{v}{}", &old[10..]) } else { v }
                        };
                        commit_text(out)
                    }/>
            }
            .into_any()
        }
        Kind::Rich => {
            let declared = f.with_value(|f| f.options.get("format").and_then(Value::as_str).map(str::to_lowercase));
            let t = text();
            let format = match declared.as_deref() {
                Some("html") => Format::Html,
                Some("markdown") => Format::Markdown,
                _ if t.trim().is_empty() || looks_like_html(&t) => Format::Html,
                _ => Format::Markdown,
            };
            view! {
                <RichText value=Signal::derive(text) format=format
                    on_commit=Callback::new(move |v: String| commit_text(v))/>
            }
            .into_any()
        }
        Kind::LongText => view! {
            <textarea class="input cms-textarea" rows="4" prop:value=text
                on:change=move |e| commit_text(event_target_value(&e))></textarea>
        }
        .into_any(),
        Kind::Code => view! {
            <textarea class="input cms-textarea cms-json" rows="6" spellcheck="false" prop:value=text
                on:change=move |e| commit_text(event_target_value(&e))></textarea>
        }
        .into_any(),
        Kind::Json => view! {
            <textarea class="input cms-textarea cms-json" rows="5" spellcheck="false" prop:value=text
                on:change=move |e| {
                    let t = event_target_value(&e);
                    if t.trim().is_empty() {
                        return commit.run(None);
                    }
                    match serde_json::from_str::<Value>(&t) {
                        Ok(v) => commit.run(Some(v)),
                        Err(err) => error.set(Some(ui.now("cms-json", &[("detail", err.to_string())]))),
                    }
                }></textarea>
        }
        .into_any(),
        Kind::List => view! {
            <TagsInput items=Signal::derive(strings) set=Callback::new(write_strings)/>
        }
        .into_any(),
        Kind::Image => {
            // Fichier du site : aperçu, adresse, choix parmi les médias.
            let images = f.with_value(|f| f.kind.as_deref() != Some("file"));
            let base = crate::library::canvas_base(ui, &ui.site_path().unwrap_or_default());
            let thumb = move || {
                let t = text();
                let image = images || is_image(&t);
                (image && !t.trim().is_empty())
                    .then(|| base.get().map(|b| crate::library::thumb_src(&b, t.trim())))
                    .flatten()
            };
            view! {
                <span class="field-line cms-media">
                    {move || thumb().map(|src| view! { <img class="cms-thumb" src=src alt=""/> })}
                    <input class="input" type="text" spellcheck="false" prop:value=text
                        on:change=move |e| commit_text(event_target_value(&e))/>
                    <button type="button" class="icon-btn icon-btn--small"
                        title=ui.t(if images { "picker-choose-image" } else { "picker-choose-media" })
                        aria-label=ui.t(if images { "picker-choose-image" } else { "picker-choose-media" })
                        on:click=move |_| crate::library::pick_media(ui, images, None, commit_text)>
                        {crate::icons::image()}</button>
                </span>
            }
            .into_any()
        }
        Kind::Text => view! {
            <input class="input" type="text" spellcheck="true" prop:value=text
                on:change=move |e| commit_text(event_target_value(&e))/>
        }
        .into_any(),
        Kind::Select { multiple: false } => {
            let options = Signal::derive(move || f.with_value(choices));
            view! { <SelectOne options=options value=Signal::derive(text) set=Callback::new(commit_text)/> }.into_any()
        }
        Kind::Select { multiple: true } => {
            let options = f.with_value(choices);
            let max = f.with_value(|f| f.options.get("max").and_then(Value::as_u64)).map(|m| m as usize);
            view! {
                <div class="choice-chips" role="group">
                    {move || {
                        let on = strings();
                        let mut list = options.clone();
                        for v in &on {
                            if !list.iter().any(|(x, _)| x == v) {
                                list.push((v.clone(), v.clone()));
                            }
                        }
                        list.into_iter().map(|(v, l)| {
                            let is_on = on.contains(&v);
                            let full = !is_on && max.is_some_and(|m| on.len() >= m);
                            view! {
                                <button type="button" class="chip chip--choice" class:is-on=is_on
                                    aria-pressed=is_on.to_string() disabled=full
                                    on:click=move |_| {
                                        let mut cur = strings();
                                        if let Some(i) = cur.iter().position(|x| *x == v) { cur.remove(i); } else { cur.push(v.clone()); }
                                        write_strings(cur)
                                    }>{l}</button>
                            }
                        }).collect_view()
                    }}
                </div>
            }
            .into_any()
        }
        Kind::Reference { multiple: false } => {
            let options = Signal::derive(move || {
                cols.with(|c| f.with_value(|f| reference_choices(c.as_deref().unwrap_or_default(), f)))
            });
            view! { <SelectOne options=options value=Signal::derive(text) set=Callback::new(commit_text)/> }.into_any()
        }
        Kind::Reference { multiple: true } => {
            let options = Memo::new(move |_| {
                cols.with(|c| f.with_value(|f| reference_choices(c.as_deref().unwrap_or_default(), f)))
            });
            let max = f.with_value(|f| f.options.get("max").and_then(Value::as_u64)).map(|m| m as usize);
            let label = move |v: &str| {
                options.with(|o| o.iter().find(|(x, _)| x == v).map(|(_, l)| l.clone())).unwrap_or_else(|| v.to_string())
            };
            view! {
                <div class="tags-input cms-refs">
                    {move || strings().into_iter().enumerate().map(|(i, v)| view! {
                        <span class="chip chip--tag" title=v.clone()>{label(&v)}
                            <button type="button" class="chip-x" title=ui.t("action-delete") aria-label=ui.t("action-delete")
                                on:click=move |_| {
                                    let mut cur = strings();
                                    if i < cur.len() { cur.remove(i); }
                                    write_strings(cur)
                                }>{icons::close()}</button>
                        </span>
                    }).collect_view()}
                    {move || {
                        let on = strings();
                        let rest: Vec<(String, String)> = options.get().into_iter().filter(|(v, _)| !on.contains(v)).collect();
                        (!rest.is_empty() && max.is_none_or(|m| on.len() < m)).then(|| view! {
                            <select class="tags-pick" aria-label=ui.t("cms-tag-add")
                                on:change=move |e| {
                                    let sel = event_target::<web_sys::HtmlSelectElement>(&e);
                                    let v = sel.value();
                                    sel.set_value("");
                                    if v.is_empty() { return; }
                                    let mut cur = strings();
                                    if !cur.contains(&v) { cur.push(v); }
                                    write_strings(cur)
                                }>
                                <option value="" selected=true>{ui.t("cms-tag-add")}</option>
                                {rest.into_iter().map(|(v, l)| view! { <option value=v>{l}</option> }).collect_view()}
                            </select>
                        })
                    }}
                </div>
            }
            .into_any()
        }
        Kind::Object | Kind::ObjectList | Kind::Blocks => ().into_any(),
    }
}

/// Liste déroulante d'une valeur ; une valeur absente des choix reste affichée.
#[component]
fn SelectOne(
    options: Signal<Vec<(String, String)>>,
    value: Signal<String>,
    set: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="select">
            <select class="input" on:change=move |e| set.run(event_target_value(&e))>
                {move || {
                    let cur = value.get();
                    let mut list = options.get();
                    if !cur.is_empty() && !list.iter().any(|(v, _)| *v == cur) {
                        list.insert(0, (cur.clone(), cur.clone()));
                    }
                    let empty = cur.is_empty();
                    let first = view! { <option value="" prop:selected=empty>"—"</option> };
                    let rest = list.into_iter().map(|(v, l)| {
                        let on = v == cur;
                        view! { <option value=v prop:selected=on>{l}</option> }
                    }).collect_view();
                    view! { {first}{rest} }
                }}
            </select>
        </div>
    }
}

/// Liste de textes : pastilles (× pour retirer), champ pour ajouter (Entrée).
#[component]
fn TagsInput(items: Signal<Vec<String>>, set: Callback<Vec<String>>) -> impl IntoView {
    let ui = expect_context::<Ui>();
    view! {
        <div class="tags-input">
            {move || items.get().into_iter().enumerate().map(|(i, t)| view! {
                <span class="chip chip--tag">{t}
                    <button type="button" class="chip-x" title=ui.t("action-delete") aria-label=ui.t("action-delete")
                        on:click=move |e| {
                            e.prevent_default();
                            let mut v = items.get_untracked();
                            if i < v.len() { v.remove(i); }
                            set.run(v);
                        }>{icons::close()}</button>
                </span>
            }).collect_view()}
            <input class="tags-add" type="text" placeholder=ui.t("cms-tag-add")
                on:keydown=move |e: ev::KeyboardEvent| {
                    if e.key() != "Enter" { return; }
                    e.prevent_default();
                    let input = event_target::<web_sys::HtmlInputElement>(&e);
                    let t = input.value().trim().to_string();
                    if t.is_empty() { return; }
                    input.set_value("");
                    let mut v = items.get_untracked();
                    if !v.contains(&t) { v.push(t); }
                    set.run(v);
                }/>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_de_date() {
        let p = parse_date("yyyy-MM-dd'T'HH:mm:ss", "2024-05-01T10:30:05").unwrap();
        assert_eq!((p.y, p.mo, p.d, p.h, p.mi, p.s), (2024, 5, 1, 10, 30, 5));
        assert_eq!(format_date("yyyy-MM-dd'T'HH:mm", p), "2024-05-01T10:30");
        assert_eq!(format_date("dd/MM/yy", p), "01/05/24");
        assert_eq!(format_date("d 'h''' H", p), "1 h' 10");
        assert!(parse_date("yyyy-MM-dd", "2024-05-01T10:30").is_none());
        assert_eq!(iso_parts("2024-05-01 10:30:00Z").map(|p| p.mi), Some(30));
        assert_eq!(iso_parts("2024-05-01").map(|p| (p.d, p.h)), Some((1, 0)));
    }

    #[test]
    fn modeles() {
        let item = serde_json::json!({ "name": "a.md", "fields": { "slug": "a", "seo": { "title": "T" } } });
        assert_eq!(
            interpolate("{fields.slug}|{slug}|{name}|{seo.title}|{x}", &item),
            "a|a|a.md|T|"
        );
        let f = CmsField {
            name: "s".into(),
            kind: Some("select".into()),
            options: serde_json::json!({ "values": ["a", { "name": "b", "label": "Bé" }, { "value": 3 }] }),
            ..Default::default()
        };
        assert_eq!(
            choices(&f),
            vec![
                ("a".into(), "a".into()),
                ("b".into(), "Bé".into()),
                ("3".into(), "3".into())
            ]
        );
    }
}

/// Valeur d'un champ d'une définition, à la création d'un élément.
fn defaults_of(fields: &[CmsField]) -> serde_json::Map<String, Value> {
    fields
        .iter()
        .filter_map(|f| f.default.clone().map(|d| (f.name.clone(), d)))
        .collect()
}

/// Liste d'objets ou blocs : une carte par élément (type, monter, descendre, supprimer, champs),
/// ajout en bas (choix du type pour des blocs).
fn repeat_view(
    ui: Ui,
    f: CmsField,
    kind: Kind,
    value: Signal<Option<Value>>,
    set: Callback<Option<Value>>,
    cols: Cols,
    head: AnyView,
) -> AnyView {
    let name = f.name.clone();
    let blocks = kind == Kind::Blocks;
    let key = StoredValue::new(
        f.options
            .get("blockKey")
            .and_then(Value::as_str)
            .unwrap_or("_block")
            .to_string(),
    );
    // Blocs sans `list` : un seul bloc (objet).
    let single = blocks && !f.list;
    let fs = StoredValue::new(f);
    let items = move || -> Vec<Value> {
        match value.get() {
            Some(Value::Array(a)) => a,
            Some(v @ Value::Object(_)) => vec![v],
            _ => Vec::new(),
        }
    };
    let write = move |list: Vec<Value>| {
        if single {
            set.run(list.into_iter().next());
        } else {
            set.run((!list.is_empty()).then_some(Value::Array(list)));
        }
    };
    let current = move || -> Vec<Value> {
        match value.get_untracked() {
            Some(Value::Array(a)) => a,
            Some(v @ Value::Object(_)) => vec![v],
            _ => Vec::new(),
        }
    };
    let def_of = move |item: &Value| -> Option<CmsField> {
        if !blocks {
            return None;
        }
        let t = item
            .get(key.get_value())
            .and_then(Value::as_str)?
            .to_string();
        fs.with_value(|f| f.fields.iter().find(|b| b.name == t).cloned())
    };
    let add = move |e: leptos::ev::MouseEvent| {
        if blocks {
            use crate::ctxmenu::{CtxItem, open};
            let defs = fs.with_value(|f| f.fields.clone());
            let items: Vec<CtxItem> = defs
                .into_iter()
                .map(|d| {
                    let label = label_of(&d);
                    CtxItem::new(label, move || {
                        let mut o = defaults_of(&d.fields);
                        o.insert(key.get_value(), Value::String(d.name.clone()));
                        let mut list = if single { Vec::new() } else { current() };
                        list.push(Value::Object(o));
                        write(list);
                    })
                })
                .collect();
            open(ui, &e, items);
        } else {
            let o = fs.with_value(|f| defaults_of(&f.fields));
            let mut list = current();
            list.push(Value::Object(o));
            write(list);
        }
    };
    view! {
        <div class="cms-item-field cms-repeat" data-field=name>
            {head}
            {move || {
                let list = items();
                let n = list.len();
                list.into_iter().enumerate().map(|(i, item)| {
                    let def = def_of(&item);
                    let title = if blocks {
                        def.as_ref().map(label_of).unwrap_or_else(|| {
                            item.get(key.get_value()).and_then(Value::as_str).unwrap_or("?").to_string()
                        })
                    } else {
                        format!("{}", i + 1)
                    };
                    let subs: Option<Vec<CmsField>> = if blocks {
                        def.map(|d| d.fields)
                    } else {
                        Some(fs.with_value(|f| f.fields.clone()))
                    };
                    let move_to = move |to: usize| {
                        let mut list = current();
                        if to < list.len() && i < list.len() {
                            let it = list.remove(i);
                            list.insert(to, it);
                            write(list);
                        }
                    };
                    let remove = move |_| {
                        let mut list = current();
                        if i < list.len() {
                            list.remove(i);
                            write(list);
                        }
                    };
                    let body = match subs {
                        Some(subs) => subs.into_iter().filter(|s| !s.hidden).map(|s| {
                            let k = StoredValue::new(s.name.clone());
                            let sub_value = Signal::derive(move || {
                                let list = match value.get() {
                                    Some(Value::Array(a)) => a,
                                    Some(v @ Value::Object(_)) => vec![v],
                                    _ => Vec::new(),
                                };
                                list.get(i).and_then(|o| o.get(k.get_value()).cloned())
                            });
                            let sub_set = Callback::new(move |v: Option<Value>| {
                                let mut list = current();
                                let Some(Value::Object(o)) = list.get_mut(i) else { return };
                                match v {
                                    Some(v) => { o.insert(k.get_value(), v); }
                                    None => { o.remove(&k.get_value()); }
                                }
                                write(list);
                            });
                            field_row(s, sub_value, sub_set, cols, None)
                        }).collect_view().into_any(),
                        // Type de bloc inconnu du schéma : modifié en JSON.
                        None => {
                            let text = serde_json::to_string_pretty(&item).unwrap_or_default();
                            view! {
                                <textarea class="input cms-textarea cms-json" rows="5" spellcheck="false" prop:value=text
                                    on:change=move |e| {
                                        if let Ok(v) = serde_json::from_str::<Value>(&event_target_value(&e)) {
                                            let mut list = current();
                                            if i < list.len() {
                                                list[i] = v;
                                                write(list);
                                            }
                                        }
                                    }></textarea>
                            }.into_any()
                        }
                    };
                    view! {
                        <div class="cms-repeat-item" data-index=i>
                            <div class="cms-repeat-head">
                                <span class="cms-repeat-title">{title}</span>
                                <button type="button" class="icon-btn icon-btn--small" title=ui.t("cms-repeat-up") aria-label=ui.t("cms-repeat-up")
                                    disabled={i == 0} on:click=move |_| move_to(i.saturating_sub(1))>{icons::arrow_up()}</button>
                                <button type="button" class="icon-btn icon-btn--small" title=ui.t("cms-repeat-down") aria-label=ui.t("cms-repeat-down")
                                    disabled={i + 1 >= n} on:click=move |_| move_to(i + 1)>{icons::arrow_down()}</button>
                                <button type="button" class="icon-btn icon-btn--small" title=ui.t("cms-repeat-remove") aria-label=ui.t("cms-repeat-remove")
                                    on:click=remove>{icons::trash()}</button>
                            </div>
                            <div class="cms-repeat-body">{body}</div>
                        </div>
                    }
                }).collect_view()
            }}
            {move || (!single || items().is_empty()).then(|| view! {
                <button type="button" class="btn btn--small cms-repeat-add" on:click=add>
                    {icons::plus()}{ui.t(if blocks { "cms-repeat-add-block" } else { "cms-repeat-add" })}</button>
            })}
        </div>
    }
    .into_any()
}
