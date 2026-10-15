//! Traduction automatique hors ligne (D-36) : textes pas encore traduits d'une langue, traduits
//! par lots sur cet ordinateur par les modèles livrés avec Revel (aucun service extérieur, aucune
//! connexion). Chaque lot est écrit aussitôt et marqué « à relire » ; toute l'opération forme une
//! seule étape d'annulation. L'opération continue quand on quitte la vue Localisation.

use crate::{Ui, api};
use leptos::prelude::*;
use leptos::task::spawn_local;
use revel_edit::Command;
use revel_edit::canvas::{Binding, Seg, Source};
use revel_protocol::{CmsField, TranslateItem};
use serde_json::Value;

/// Opération en cours (une à la fois).
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    /// Site traduit.
    pub path: String,
    /// Langue traduite.
    pub code: String,
    /// Textes et contenus traités, sur `total`.
    pub done: usize,
    pub total: usize,
    /// Caractères traduits et à traduire (estimation du temps restant).
    pub chars_done: usize,
    pub chars_total: usize,
    /// Début (millisecondes).
    pub started: f64,
    /// Arrêt demandé : le lot en cours est écrit, puis l'opération s'arrête.
    pub stop: bool,
}

impl Job {
    /// Temps restant estimé, en secondes (après le premier lot).
    pub fn remaining(&self) -> Option<u64> {
        if self.chars_done == 0 {
            return None;
        }
        let spent = (js_sys::Date::now() - self.started) / 1000.0;
        let left = self.chars_total.saturating_sub(self.chars_done) as f64;
        Some((spent / self.chars_done as f64 * left).round() as u64)
    }
}

/// Texte à traduire.
#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    /// Texte écrit dans les pages (`Locale::texts`).
    Text { source: String },
    /// Valeur d'un fichier de données (écrite dans le dossier de la langue).
    Data { path: Vec<Seg>, text: String },
    /// Contenu d'une collection (copié dans le dossier de la langue, champs texte traduits).
    Entry {
        /// Dossier de la collection dans la langue principale.
        folder: String,
        file: String,
        data: Value,
        body: String,
        fields: Vec<CmsField>,
    },
}

/// Lot de textes simples traduits en un appel.
const BATCH_ITEMS: usize = 8;
const BATCH_CHARS: usize = 900;

/// Clés d'en-tête jamais traduites sans schéma (identifiants, adresses, fichiers).
const SKIP_KEYS: &[&str] = &[
    "slug",
    "id",
    "_block",
    "type",
    "template",
    "layout",
    "permalink",
    "url",
    "href",
    "src",
    "image",
    "cover",
    "poster",
    "video",
    "file",
    "icon",
    "color",
    "class",
    "date",
    "lang",
    "ref",
    "key",
    "tags",
];

/// Étape d'un chemin dans l'en-tête d'un contenu.
#[derive(Debug, Clone, PartialEq)]
enum Step {
    Key(String),
    At(usize),
}

/// Texte trouvé dans l'en-tête : chemin, texte, forme.
type Found = (Vec<Step>, String, Option<&'static str>);

/// Forme d'un texte d'après son champ ; texte riche sans format indiqué : devinée.
fn text_format(f: &CmsField) -> Option<&'static str> {
    match f.kind.as_deref() {
        Some("rich_text") => match f.options.get("format").and_then(Value::as_str) {
            Some("html") => Some("html"),
            Some("markdown") => Some("markdown"),
            _ => None,
        },
        _ => Some("plain"),
    }
}

/// Textes à traduire d'une valeur, d'après son champ quand le schéma le décrit.
fn walk(field: Option<&CmsField>, key: &str, v: &Value, at: &mut Vec<Step>, out: &mut Vec<Found>) {
    if let Some(f) = field {
        if f.hidden || f.pattern.is_some() || f.name == "slug" {
            return;
        }
        match f.kind.as_deref() {
            Some("string" | "text" | "rich_text") => {
                strings(v, text_format(f), at, out);
                return;
            }
            Some("object") => {
                if let Value::Object(m) = v {
                    walk_object(&f.fields, m, None, at, out);
                }
                return;
            }
            Some("block") => {
                blocks(f, v, at, out);
                return;
            }
            None => {}
            _ => return,
        }
    } else if SKIP_KEYS.contains(&key) {
        return;
    }
    match v {
        Value::String(s) => {
            if revel_edit::locale::text_like(s) {
                out.push((at.clone(), s.clone(), None));
            }
        }
        Value::Array(list) => {
            for (i, x) in list.iter().enumerate() {
                at.push(Step::At(i));
                walk(field, key, x, at, out);
                at.pop();
            }
        }
        Value::Object(m) => {
            let nested = field.map(|f| f.fields.clone()).unwrap_or_default();
            walk_object(&nested, m, None, at, out);
        }
        _ => {}
    }
}

/// Blocs de Pages CMS : chaque élément est décrit par le bloc que nomme sa clé de type
/// (`blockKey`, `_block` par défaut), clé jamais traduite.
fn blocks(f: &CmsField, v: &Value, at: &mut Vec<Step>, out: &mut Vec<Found>) {
    let key = f
        .options
        .get("blockKey")
        .and_then(Value::as_str)
        .unwrap_or("_block");
    let one = |item: &Value, at: &mut Vec<Step>, out: &mut Vec<Found>| {
        if let Value::Object(m) = item {
            let def = m
                .get(key)
                .and_then(Value::as_str)
                .and_then(|t| f.fields.iter().find(|d| d.name == t));
            let fields = def.map(|d| d.fields.clone()).unwrap_or_default();
            walk_object(&fields, m, Some(key), at, out);
        }
    };
    match v {
        Value::Array(list) => {
            for (i, item) in list.iter().enumerate() {
                at.push(Step::At(i));
                one(item, at, out);
                at.pop();
            }
        }
        other => one(other, at, out),
    }
}

fn walk_object(
    fields: &[CmsField],
    m: &serde_json::Map<String, Value>,
    skip: Option<&str>,
    at: &mut Vec<Step>,
    out: &mut Vec<Found>,
) {
    for (k, v) in m {
        if skip == Some(k.as_str()) {
            continue;
        }
        at.push(Step::Key(k.clone()));
        walk(fields.iter().find(|f| &f.name == k), k, v, at, out);
        at.pop();
    }
}

/// Textes d'une valeur de type texte (seule ou en liste).
fn strings(v: &Value, format: Option<&'static str>, at: &mut Vec<Step>, out: &mut Vec<Found>) {
    match v {
        Value::String(s) if revel_edit::locale::translatable(s) => {
            out.push((at.clone(), s.clone(), format))
        }
        Value::Array(list) => {
            for (i, x) in list.iter().enumerate() {
                at.push(Step::At(i));
                strings(x, format, at, out);
                at.pop();
            }
        }
        _ => {}
    }
}

fn set_at(v: &mut Value, at: &[Step], text: String) {
    let Some((first, rest)) = at.split_first() else {
        *v = Value::String(text);
        return;
    };
    let next = match (first, v) {
        (Step::Key(k), Value::Object(m)) => m.get_mut(k),
        (Step::At(i), Value::Array(l)) => l.get_mut(*i),
        _ => None,
    };
    if let Some(n) = next {
        set_at(n, rest, text);
    }
}

/// Textes d'un contenu à traduire (en-tête puis texte).
fn entry_found(data: &Value, body: &str, fields: &[CmsField]) -> Vec<Found> {
    let mut out = Vec::new();
    if let Value::Object(m) = data {
        walk_object(fields, m, None, &mut Vec::new(), &mut out);
    }
    if revel_edit::locale::translatable(body.trim()) {
        out.push((
            vec![Step::Key(String::new())],
            body.to_string(),
            Some("markdown"),
        ));
    }
    out
}

/// Caractères à traduire d'un texte (estimation du temps).
pub fn unit_chars(u: &Unit) -> usize {
    match u {
        Unit::Text { source } => source.chars().count(),
        Unit::Data { text, .. } => text.chars().count(),
        Unit::Entry {
            data, body, fields, ..
        } => entry_found(data, body, fields)
            .iter()
            .map(|f| f.1.chars().count())
            .sum(),
    }
}

/// Démarre la traduction automatique de `units` de la langue `from` vers `code`.
pub fn start(
    ui: Ui,
    path: String,
    from: String,
    code: String,
    units: Vec<Unit>,
    keep: Vec<String>,
) {
    if units.is_empty() || ui.auto.with_untracked(Option::is_some) {
        return;
    }
    let chars_total = units.iter().map(unit_chars).sum();
    let started = js_sys::Date::now();
    ui.auto.set(Some(Job {
        path: path.clone(),
        code: code.clone(),
        done: 0,
        total: units.len(),
        chars_done: 0,
        chars_total,
        started,
        stop: false,
    }));
    ui.activity.set(Some(0.02));
    let group = format!("auto-{code}-{started}");
    spawn_local(async move {
        let result = run(ui, &path, &from, &code, &units, &keep, &group).await;
        ui.auto.set(None);
        ui.activity.set(None);
        if let Err(e) = result {
            ui.fail(e);
        }
    });
}

/// Arrêt demandé, ou site fermé.
fn stopped(ui: Ui) -> bool {
    let site = ui.site_path();
    ui.auto.with_untracked(|j| {
        j.as_ref()
            .is_none_or(|j| j.stop || site.as_deref() != Some(j.path.as_str()))
    })
}

fn advance(ui: Ui, n: usize, chars: usize) {
    ui.auto.update(|j| {
        if let Some(j) = j {
            j.done += n;
            j.chars_done += chars;
        }
    });
    let f = ui.auto.with_untracked(|j| {
        j.as_ref()
            .map(|j| j.chars_done as f32 / j.chars_total.max(1) as f32)
    });
    if let Some(f) = f {
        ui.activity.set(Some(f.clamp(0.02, 1.0)));
    }
}

async fn run(
    ui: Ui,
    path: &str,
    from: &str,
    code: &str,
    units: &[Unit],
    keep: &[String],
    group: &str,
) -> Result<(), revel_protocol::Failure> {
    let mut i = 0;
    while i < units.len() {
        if stopped(ui) {
            break;
        }
        if let Unit::Entry {
            folder,
            file,
            data,
            body,
            fields,
        } = &units[i]
        {
            let chars = unit_chars(&units[i]);
            if let Some(cmd) = entry(from, code, folder, file, data, body, fields, keep).await? {
                api::edit_apply_group(path, &cmd, group).await?;
            }
            advance(ui, 1, chars);
            i += 1;
            continue;
        }
        // Lot de textes simples (pages, données).
        let mut j = i;
        let mut chars = 0;
        while j < units.len()
            && j - i < BATCH_ITEMS
            && (j == i || chars < BATCH_CHARS)
            && !matches!(units[j], Unit::Entry { .. })
        {
            chars += unit_chars(&units[j]);
            j += 1;
        }
        let batch = &units[i..j];
        let items: Vec<TranslateItem> = batch
            .iter()
            .map(|u| match u {
                Unit::Text { source } => TranslateItem {
                    text: source.clone(),
                    format: Some("plain".into()),
                },
                Unit::Data { text, .. } => TranslateItem {
                    text: text.clone(),
                    format: None,
                },
                Unit::Entry { .. } => unreachable!(),
            })
            .collect();
        let out = api::translate(from, code, &items, keep).await?;
        let mut commands = Vec::new();
        let mut marks = Vec::new();
        for (u, t) in batch.iter().zip(out) {
            if t.trim().is_empty() {
                continue;
            }
            match u {
                Unit::Text { source } => commands.push(Command::SetTranslation {
                    locale: code.to_string(),
                    source: source.clone(),
                    text: Some(t),
                    machine: true,
                }),
                Unit::Data { path: p, .. } => {
                    let binding = Binding {
                        source: Source::LocaleData {
                            locale: code.to_string(),
                        },
                        path: p.clone(),
                    };
                    // Clé absente du fichier de la langue : créée (chemins sans liste).
                    commands.push(if p.iter().all(|s| matches!(s, Seg::Key(_))) {
                        Command::SetDataValue {
                            binding,
                            value: Some(Value::String(t)),
                        }
                    } else {
                        Command::SetBound { binding, value: t }
                    });
                    marks.push(revel_edit::locale::data_key(p));
                }
                Unit::Entry { .. } => {}
            }
        }
        if !marks.is_empty() {
            commands.push(Command::MarkMachine {
                locale: code.to_string(),
                keys: marks,
                on: true,
            });
        }
        if !commands.is_empty() {
            api::edit_apply_group(path, &Command::Batch { commands }, group).await?;
        }
        advance(ui, j - i, chars);
        i = j;
    }
    Ok(())
}

/// Contenu traduit : commandes qui écrivent le contenu de la langue (`None` : rien à traduire).
#[allow(clippy::too_many_arguments)]
async fn entry(
    from: &str,
    code: &str,
    folder: &str,
    file: &str,
    data: &Value,
    body: &str,
    fields: &[CmsField],
    keep: &[String],
) -> Result<Option<Command>, revel_protocol::Failure> {
    let found = entry_found(data, body, fields);
    if found.is_empty() {
        return Ok(None);
    }
    // Appels de taille limitée (un long contenu en plusieurs fois) ; un contenu commencé est
    // terminé avant l'arrêt demandé.
    let mut done: Vec<String> = Vec::with_capacity(found.len());
    let mut k = 0;
    while k < found.len() {
        let mut end = k;
        let mut chars = 0;
        while end < found.len() && (end == k || chars + found[end].1.len() < 1500) {
            chars += found[end].1.len();
            end += 1;
        }
        let items: Vec<TranslateItem> = found[k..end]
            .iter()
            .map(|(_, t, f)| TranslateItem {
                text: t.clone(),
                format: f.map(str::to_string),
            })
            .collect();
        done.extend(api::translate(from, code, &items, keep).await?);
        k = end;
    }
    let mut new = data.clone();
    let mut new_body = None;
    for ((at, src, _), t) in found.iter().zip(done) {
        if t.trim().is_empty() {
            continue;
        }
        if at.len() == 1 && at[0] == Step::Key(String::new()) {
            // Texte du contenu : espaces de fin d'origine gardés.
            let tail = &src[src.trim_end().len()..];
            new_body = Some(format!("{}{tail}", t.trim_end()));
        } else {
            set_at(&mut new, at, t);
        }
    }
    let target = revel_model::locale_folder(folder, code);
    let mut commands = Vec::new();
    if let (Value::Object(old), Value::Object(now)) = (data, &new) {
        for (key, v) in now {
            if old.get(key) != Some(v) {
                commands.push(Command::SetEntryField {
                    folder: target.clone(),
                    file: file.to_string(),
                    key: key.clone(),
                    value: Some(v.clone()),
                });
            }
        }
    }
    if let Some(b) = new_body {
        commands.push(Command::SetEntryBody {
            folder: target,
            file: file.to_string(),
            body: b,
        });
    }
    if commands.is_empty() {
        return Ok(None);
    }
    commands.push(Command::MarkMachine {
        locale: code.to_string(),
        keys: vec![revel_edit::locale::entry_key(folder, file)],
        on: true,
    });
    Ok(Some(Command::Batch { commands }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, kind: &str) -> CmsField {
        CmsField {
            name: name.into(),
            kind: Some(kind.into()),
            ..Default::default()
        }
    }

    #[test]
    fn textes_d_un_contenu() {
        let mut intro = field("intro", "rich_text");
        intro.options = serde_json::json!({ "format": "html" });
        let mut seo = field("seo", "object");
        seo.fields = vec![field("title", "string"), field("image", "image")];
        let mut quote = field("quote", "object");
        quote.fields = vec![field("quote", "rich_text")];
        let mut blocks = field("blocks", "block");
        blocks.options = serde_json::json!({ "blockKey": "type" });
        blocks.fields = vec![quote];
        let fields = vec![
            blocks,
            field("title", "string"),
            field("slug", "string"),
            field("cover", "image"),
            field("format", "select"),
            intro,
            seo,
        ];
        let data = serde_json::json!({
            "title": "Un film",
            "slug": "un-film",
            "cover": "/media/a.jpg",
            "format": "film",
            "intro": "<p>Bonjour</p>",
            "seo": { "title": "Le film", "image": "/media/b.jpg" },
            "blocks": [{ "type": "texte", "body": "Un texte libre" }, { "src": "/v.mp4" }, { "type": "quote", "quote": "<p>Une citation</p>" }]
        });
        let found = entry_found(&data, "Corps du **contenu**\n", &fields);
        let texts: Vec<(&str, Option<&str>)> = found.iter().map(|f| (f.1.as_str(), f.2)).collect();
        assert!(texts.contains(&("Un film", Some("plain"))));
        assert!(texts.contains(&("<p>Bonjour</p>", Some("html"))));
        assert!(texts.contains(&("Le film", Some("plain"))));
        assert!(texts.contains(&("Un texte libre", None)));
        assert!(texts.contains(&("Corps du **contenu**\n", Some("markdown"))));
        assert!(texts.contains(&("<p>Une citation</p>", None)));
        assert_eq!(found.len(), 6);
        // Valeur remise à sa place.
        let mut v = data.clone();
        let at = &found.iter().find(|f| f.1 == "Le film").unwrap().0;
        set_at(&mut v, at, "The film".into());
        assert_eq!(v["seo"]["title"], "The film");
    }
}
