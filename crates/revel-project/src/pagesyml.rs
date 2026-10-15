//! Configuration de Pages CMS (`.pages.yml`) modifiée par Revel (étape 3.3, D-27) : champs d'une
//! collection ajoutés, modifiés, retirés, déplacés ; réglages d'une collection ; collection
//! ajoutée.
//!
//! Comme pour les contenus, seule la partie visée du texte est réécrite — une clé d'un champ, un
//! champ de la liste, la liste des champs, au pire la collection — et le reste (commentaires,
//! mise en forme, autres collections) est gardé octet pour octet. Chaque résultat est relu et
//! comparé au document voulu ; sinon la réécriture plus large suivante est essayée, et l'écriture
//! est refusée si aucune ne convient.

use crate::bound::{BoundError, write_atomic};
use crate::entries::{emit_entry, emit_item, key_of, same};
use serde_json::{Map, Value};
use std::path::Path;

/// Nom du fichier de configuration lu par Pages CMS, à la racine du site.
pub const CONFIG_FILE: &str = ".pages.yml";

/// En-tête d'un fichier créé par Revel.
const HEADER: &str = "# Configuration de Pages CMS (https://pagescms.org/docs/configuration/), tenue à jour par Revel.\n";

/// Modification fusionnée dans une définition (comme RFC 7386) : objets fusionnés clé par clé,
/// `null` retire la clé, un objet vidé est retiré, le reste (listes comprises) remplacé.
pub fn merge(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
    for (k, v) in patch {
        match v {
            Value::Null => {
                target.remove(k);
            }
            Value::Object(p) => {
                let mut cur = target
                    .get(k)
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                merge(&mut cur, p);
                if cur.is_empty() {
                    target.remove(k);
                } else {
                    target.insert(k.clone(), Value::Object(cur));
                }
            }
            v => {
                target.insert(k.clone(), v.clone());
            }
        }
    }
}

/// Opération sur la liste des champs d'une collection.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldOp {
    /// Clés d'un champ modifiées, fusionnées (`merge`) ; son nom ne change pas.
    Set {
        field: String,
        patch: Map<String, Value>,
    },
    /// Nouveau champ (définition complète), à la position donnée (sinon à la fin).
    Add {
        def: Value,
        index: Option<usize>,
    },
    Remove {
        field: String,
    },
    /// Champ déplacé à la position donnée (dans la liste finale).
    Move {
        field: String,
        index: usize,
    },
}

// ------------------------------------------------------------------ Lignes et blocs

#[derive(Debug, Clone, Copy)]
struct Ln {
    start: usize,
    end: usize,
    indent: usize,
    blank: bool,
    comment: bool,
    dash: bool,
}

fn scan(text: &str) -> Vec<Ln> {
    let mut out = Vec::new();
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let t = line.trim_end_matches(['\n', '\r']);
        let body = t.trim_start_matches(' ');
        let blank = body.trim().is_empty();
        out.push(Ln {
            start: pos,
            end: pos + line.len(),
            indent: t.len() - body.len(),
            blank,
            comment: !blank && body.starts_with('#'),
            dash: body == "-" || body.starts_with("- "),
        });
        pos += line.len();
    }
    out
}

/// Fin (exclue) d'un bloc commencé à `start` : les lignes vides et les commentaires qui le
/// terminent (retrait <= `indent`) appartiennent au bloc suivant.
fn trim_end(ls: &[Ln], start: usize, mut end: usize, indent: usize) -> usize {
    while end > start + 1
        && (ls[end - 1].blank || (ls[end - 1].comment && ls[end - 1].indent <= indent))
    {
        end -= 1;
    }
    end
}

/// Correspondance (mapping) dont les clés sont au retrait `indent` sur les lignes `[a, b)` ;
/// `dash` : ligne du tiret qui porte sa première clé (`- name: x`).
#[derive(Debug, Clone, Copy)]
struct Mapping {
    a: usize,
    b: usize,
    indent: usize,
    dash: Option<usize>,
}

/// Clé d'une correspondance : nom, ligne, fin (exclue) de son bloc.
#[derive(Debug, Clone)]
struct Key {
    name: String,
    line: usize,
    end: usize,
}

/// Texte d'une ligne de tiret après `- `.
fn after_dash<'a>(text: &'a str, l: &Ln) -> &'a str {
    let t = text[l.start..l.end].trim_end_matches(['\n', '\r']);
    t[l.indent + 1..].trim_start_matches(' ')
}

fn keys(text: &str, ls: &[Ln], m: Mapping) -> Vec<Key> {
    let mut starts: Vec<(String, usize)> = Vec::new();
    if let Some(d) = m.dash {
        let rest = after_dash(text, &ls[d]);
        if !rest.starts_with(['{', '[', '#'])
            && let Some(k) = key_of(rest)
        {
            starts.push((k, d));
        }
    }
    for (i, l) in ls.iter().enumerate().take(m.b).skip(m.a) {
        if l.indent == m.indent
            && !l.blank
            && !l.comment
            && !l.dash
            && let Some(k) = key_of(text[l.start..l.end].trim())
        {
            starts.push((k, i));
        }
    }
    (0..starts.len())
        .map(|j| {
            let (name, line) = starts[j].clone();
            let next = starts.get(j + 1).map_or(m.b, |x| x.1);
            Key {
                name,
                line,
                end: trim_end(ls, line, next, m.indent),
            }
        })
        .collect()
}

/// Élément d'une liste en bloc : ligne du tiret, fin (exclue), retrait du tiret.
#[derive(Debug, Clone, Copy)]
struct Item {
    s: usize,
    e: usize,
    d: usize,
}

/// Éléments d'une liste en bloc sur les lignes `[a, b)`.
fn items(ls: &[Ln], a: usize, b: usize) -> Vec<Item> {
    let Some(d) = (a..b).filter(|&i| ls[i].dash).map(|i| ls[i].indent).min() else {
        return Vec::new();
    };
    // Une ligne de contenu moins en retrait que les tirets : pas une liste en bloc simple.
    if (a..b).any(|i| !ls[i].blank && !ls[i].comment && ls[i].indent < d) {
        return Vec::new();
    }
    let starts: Vec<usize> = (a..b)
        .filter(|&i| ls[i].dash && ls[i].indent == d)
        .collect();
    (0..starts.len())
        .map(|j| {
            let s = starts[j];
            let next = starts.get(j + 1).copied().unwrap_or(b);
            Item {
                s,
                e: trim_end(ls, s, next, d),
                d,
            }
        })
        .collect()
}

fn item_mapping(text: &str, ls: &[Ln], it: Item) -> Mapping {
    let rest = after_dash(text, &ls[it.s]);
    if !rest.is_empty() && !rest.starts_with('#') {
        let line = text[ls[it.s].start..ls[it.s].end].trim_end_matches(['\n', '\r']);
        Mapping {
            a: it.s + 1,
            b: it.e,
            indent: line.len() - rest.len(),
            dash: Some(it.s),
        }
    } else {
        let indent = (it.s + 1..it.e)
            .find(|&i| !ls[i].blank && !ls[i].comment)
            .map_or(it.d + 2, |i| ls[i].indent);
        Mapping {
            a: it.s + 1,
            b: it.e,
            indent,
            dash: None,
        }
    }
}

fn span(ls: &[Ln], a: usize, e: usize) -> (usize, usize) {
    (ls[a].start, ls[e - 1].end)
}

/// Première ligne d'un élément avec les commentaires écrits juste au-dessus (ils le décrivent et
/// le suivent quand il est déplacé ou retiré).
fn lead(ls: &[Ln], s: usize) -> usize {
    let mut a = s;
    while a > 0 && ls[a - 1].comment {
        a -= 1;
    }
    a
}

/// Collections (`content:`) du texte.
fn collections(text: &str, ls: &[Ln]) -> Option<(Key, Vec<Item>)> {
    let top = Mapping {
        a: 0,
        b: ls.len(),
        indent: 0,
        dash: None,
    };
    let k = keys(text, ls, top)
        .into_iter()
        .find(|k| k.name == "content")?;
    let list = items(ls, k.line + 1, k.end);
    Some((k, list))
}

fn collection_item(text: &str, ls: &[Ln], ci: usize) -> Option<Item> {
    collections(text, ls)?.1.get(ci).copied()
}

/// Liste des champs (`fields:`) d'une collection : clé et éléments.
fn field_items(text: &str, ls: &[Ln], ci: usize) -> Option<(Mapping, Key, Vec<Item>)> {
    let it = collection_item(text, ls, ci)?;
    let m = item_mapping(text, ls, it);
    let k = keys(text, ls, m).into_iter().find(|k| k.name == "fields")?;
    if Some(k.line) == m.dash {
        return None;
    }
    let list = items(ls, k.line + 1, k.end);
    Some((m, k, list))
}

/// Insère `piece` (lignes terminées) à la position `at`, en terminant la ligne précédente.
fn insert(text: &str, at: usize, piece: &str) -> String {
    let mut out = String::with_capacity(text.len() + piece.len() + 1);
    out.push_str(&text[..at]);
    if at > 0 && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(piece);
    out.push_str(&text[at..]);
    out
}

/// Clé d'un élément de liste modifiée, ajoutée (à la fin de l'élément) ou retirée (`null`).
fn set_key(text: &str, it: Item, key: &str, v: &Value) -> Option<String> {
    let ls = scan(text);
    let m = item_mapping(text, &ls, it);
    if m.dash.is_some() && after_dash(text, &ls[it.s]).starts_with(['{', '[']) {
        return None;
    }
    match keys(text, &ls, m).into_iter().find(|k| k.name == key) {
        Some(k) if Some(k.line) == m.dash => {
            // Première clé, sur la ligne du tiret : le tiret reste.
            if v.is_null() {
                return None;
            }
            let (a, b) = span(&ls, k.line, k.end);
            let e = emit_entry(key, v, m.indent);
            Some(format!(
                "{}{}{}{}",
                &text[..a],
                &text[a..a + m.indent],
                &e[m.indent..],
                &text[b..]
            ))
        }
        Some(k) => {
            let (a, b) = span(&ls, k.line, k.end);
            let e = if v.is_null() {
                String::new()
            } else {
                emit_entry(key, v, m.indent)
            };
            Some(format!("{}{e}{}", &text[..a], &text[b..]))
        }
        None if v.is_null() => Some(text.to_string()),
        None => Some(insert(
            text,
            ls[it.e - 1].end,
            &emit_entry(key, v, m.indent),
        )),
    }
}

// ------------------------------------------------------------------ Opérations

fn parse(text: &str) -> Result<Value, BoundError> {
    if text.trim().is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    crate::parse_yaml(text).map_err(|e| format!("{CONFIG_FILE} illisible : {e}"))
}

fn first_ok(
    cands: impl IntoIterator<Item = Option<String>>,
    expected: &Value,
) -> Result<String, BoundError> {
    for c in cands.into_iter().flatten() {
        if let Ok(v) = parse(&c)
            && same(&v, expected)
        {
            return Ok(c);
        }
    }
    Err("écriture refusée : relecture différente".into())
}

fn name_of(v: &Value) -> Option<&str> {
    v.get("name").and_then(Value::as_str)
}

/// Index de la collection `name` dans `content`.
fn collection_index(doc: &Value, name: &str) -> Result<usize, BoundError> {
    doc.get("content")
        .and_then(Value::as_array)
        .and_then(|c| c.iter().position(|x| name_of(x) == Some(name)))
        .ok_or_else(|| format!("collection absente de {CONFIG_FILE} : {name}"))
}

/// Applique une opération sur les champs de la collection `name`.
pub fn edit_fields(text: &str, name: &str, op: &FieldOp) -> Result<String, BoundError> {
    let doc = parse(text)?;
    let ci = collection_index(&doc, name)?;
    let old: Vec<Value> = doc["content"][ci]
        .get("fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let pos = |f: &str| {
        old.iter()
            .position(|x| name_of(x) == Some(f))
            .ok_or_else(|| format!("champ absent : {f}"))
    };
    let mut fields = old.clone();
    match op {
        FieldOp::Set { field, patch } => {
            let i = pos(field)?;
            if patch
                .get("name")
                .is_some_and(|n| n.as_str() != Some(field.as_str()))
            {
                return Err("le nom d'un champ ne change pas".into());
            }
            let o = fields[i]
                .as_object_mut()
                .ok_or_else(|| format!("champ illisible : {field}"))?;
            merge(o, patch);
        }
        FieldOp::Add { def, index } => {
            let n = name_of(def)
                .filter(|n| !n.trim().is_empty())
                .ok_or("champ sans nom")?;
            if old.iter().any(|x| name_of(x) == Some(n)) {
                return Err(format!("champ déjà présent : {n}"));
            }
            fields.insert(index.unwrap_or(old.len()).min(old.len()), def.clone());
        }
        FieldOp::Remove { field } => {
            fields.remove(pos(field)?);
        }
        FieldOp::Move { field, index } => {
            let f = fields.remove(pos(field)?);
            fields.insert((*index).min(fields.len()), f);
        }
    }
    let mut expected = doc.clone();
    expected["content"][ci]["fields"] = Value::Array(fields.clone());
    let by_item = || -> Option<String> {
        let ls = scan(text);
        let (_, _, list) = field_items(text, &ls, ci)?;
        if list.len() != old.len() || list.is_empty() {
            return None;
        }
        let sp = |j: usize| span(&ls, lead(&ls, list[j].s), list[j].e);
        let d = list[0].d;
        match op {
            FieldOp::Set { field, patch } => {
                let i = pos(field).ok()?;
                let mut t = text.to_string();
                for k in patch.keys() {
                    let v = fields[i].get(k).cloned().unwrap_or(Value::Null);
                    let ls = scan(&t);
                    let it = field_items(&t, &ls, ci)?.2.get(i).copied()?;
                    t = set_key(&t, it, k, &v)?;
                }
                Some(t)
            }
            FieldOp::Add { def, index } => {
                let i = index.unwrap_or(old.len()).min(old.len());
                let at = if i < old.len() {
                    sp(i).0
                } else {
                    sp(old.len() - 1).1
                };
                Some(insert(text, at, &emit_item(def, d)))
            }
            FieldOp::Remove { field } => {
                let (a, b) = sp(pos(field).ok()?);
                Some(format!("{}{}", &text[..a], &text[b..]))
            }
            FieldOp::Move { field, index } => {
                let i = pos(field).ok()?;
                let target = (*index).min(old.len() - 1);
                if target == i {
                    return Some(text.to_string());
                }
                let (a, b) = sp(i);
                let mut piece = text[a..b].to_string();
                if !piece.ends_with('\n') {
                    piece.push('\n');
                }
                let without = format!("{}{}", &text[..a], &text[b..]);
                // Position dans le texte sans le champ.
                let at = if target > i {
                    sp(target).1 - (b - a)
                } else {
                    sp(target).0
                };
                Some(insert(&without, at, &piece))
            }
        }
    };
    let whole_list = || -> Option<String> {
        let ls = scan(text);
        let it = collection_item(text, &ls, ci)?;
        let m = item_mapping(text, &ls, it);
        let e = emit_entry("fields", &Value::Array(fields.clone()), m.indent);
        match keys(text, &ls, m).into_iter().find(|k| k.name == "fields") {
            Some(k) if Some(k.line) != m.dash => {
                let (a, b) = span(&ls, k.line, k.end);
                Some(format!("{}{e}{}", &text[..a], &text[b..]))
            }
            Some(_) => None,
            None => Some(insert(text, ls[it.e - 1].end, &e)),
        }
    };
    let whole_collection = || replace_collection(text, ci, &expected["content"][ci]);
    first_ok([by_item(), whole_list(), whole_collection()], &expected)
}

fn replace_collection(text: &str, ci: usize, def: &Value) -> Option<String> {
    let ls = scan(text);
    let it = collection_item(text, &ls, ci)?;
    let (a, b) = span(&ls, it.s, it.e);
    Some(format!(
        "{}{}{}",
        &text[..a],
        emit_item(def, it.d),
        &text[b..]
    ))
}

/// Réglages de la collection `name` (libellé, `view`, `filename`…), fusionnés (`merge`).
pub fn set_collection(
    text: &str,
    name: &str,
    patch: &Map<String, Value>,
) -> Result<String, BoundError> {
    let doc = parse(text)?;
    let ci = collection_index(&doc, name)?;
    if patch.get("name").is_some_and(|n| n.as_str() != Some(name)) {
        return Err("le nom d'une collection ne change pas".into());
    }
    let mut expected = doc.clone();
    if let Some(o) = expected["content"][ci].as_object_mut() {
        merge(o, patch);
    }
    let def = expected["content"][ci].clone();
    let by_key = || -> Option<String> {
        let mut t = text.to_string();
        for k in patch.keys() {
            let v = def.get(k).cloned().unwrap_or(Value::Null);
            let ls = scan(&t);
            let it = collection_item(&t, &ls, ci)?;
            t = set_key(&t, it, k, &v)?;
        }
        Some(t)
    };
    first_ok(
        [
            by_key(),
            replace_collection(text, ci, &expected["content"][ci]),
        ],
        &expected,
    )
}

/// Ajoute une collection (définition complète) à la fin de `content` ; texte vide : fichier créé.
pub fn add_collection(text: &str, def: &Value) -> Result<String, BoundError> {
    let doc = parse(text)?;
    let n = name_of(def)
        .filter(|n| !n.trim().is_empty())
        .ok_or("collection sans nom")?;
    if doc
        .get("content")
        .and_then(Value::as_array)
        .is_some_and(|c| c.iter().any(|x| name_of(x) == Some(n)))
    {
        return Err(format!("collection déjà présente : {n}"));
    }
    let mut expected = doc.clone();
    if !expected.is_object() {
        return Err(format!("{CONFIG_FILE} illisible"));
    }
    let list = expected
        .as_object_mut()
        .map(|o| {
            o.entry("content")
                .or_insert_with(|| Value::Array(Vec::new()))
        })
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("{CONFIG_FILE} : « content » n'est pas une liste"))?;
    list.push(def.clone());
    let all = expected["content"].clone();
    let after_last = || -> Option<String> {
        let ls = scan(text);
        let (_, list) = collections(text, &ls)?;
        let last = list.last()?;
        let at = ls[last.e - 1].end;
        Some(insert(text, at, &format!("\n{}", emit_item(def, last.d))))
    };
    let whole_key = || -> Option<String> {
        let ls = scan(text);
        let (k, _) = collections(text, &ls)?;
        let (a, b) = span(&ls, k.line, k.end);
        Some(format!(
            "{}{}{}",
            &text[..a],
            emit_entry("content", &all, 0),
            &text[b..]
        ))
    };
    let appended = || -> Option<String> {
        if text.trim().is_empty() {
            return Some(format!("{HEADER}{}", emit_entry("content", &all, 0)));
        }
        let sep = if text.ends_with("\n\n") {
            ""
        } else if text.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        Some(format!("{text}{sep}{}", emit_entry("content", &all, 0)))
    };
    first_ok([after_last(), whole_key(), appended()], &expected)
}

/// Clé de premier niveau du fichier (`settings`, `media`) modifiée ou ajoutée (`null` : retirée).
pub fn set_top(text: &str, key: &str, v: &Value) -> Result<String, BoundError> {
    let doc = parse(text)?;
    let mut expected = doc.clone();
    if let Some(o) = expected.as_object_mut() {
        if v.is_null() {
            o.remove(key);
        } else {
            o.insert(key.to_string(), v.clone());
        }
    }
    let ls = scan(text);
    let top = Mapping {
        a: 0,
        b: ls.len(),
        indent: 0,
        dash: None,
    };
    let e = if v.is_null() {
        String::new()
    } else {
        emit_entry(key, v, 0)
    };
    let cand = match keys(text, &ls, top).into_iter().find(|k| k.name == key) {
        Some(k) => {
            let (a, b) = span(&ls, k.line, k.end);
            Some(format!("{}{e}{}", &text[..a], &text[b..]))
        }
        None if v.is_null() => Some(text.to_string()),
        // Avant `content` (réglages en tête de fichier), sinon à la fin.
        None => match keys(text, &ls, top)
            .into_iter()
            .find(|k| k.name == "content")
        {
            Some(k) => Some(insert(text, ls[k.line].start, &format!("{e}\n"))),
            None => Some(insert(text, text.len(), &e)),
        },
    };
    first_ok([cand], &expected)
}

/// Texte de `.pages.yml` (`None` : absent).
pub fn read(site: &Path) -> Result<Option<String>, BoundError> {
    let p = site.join(CONFIG_FILE);
    if !p.is_file() {
        return Ok(None);
    }
    std::fs::read_to_string(&p)
        .map(Some)
        .map_err(|e| format!("{} : {e}", p.display()))
}

/// Remplace `.pages.yml` par `text` (`None` : supprimé).
pub fn write(site: &Path, text: Option<&str>) -> Result<(), BoundError> {
    let p = site.join(CONFIG_FILE);
    match text {
        Some(t) => write_atomic(&p, t),
        None => {
            if p.is_file() {
                std::fs::remove_file(&p).map_err(|e| format!("{} : {e}", p.display()))?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const CONFIG: &str = "# Configuration\nmedia:\n  input: src/media\n\ncontent:\n  # ---- Projets\n  - name: projets\n    label: Projets\n    type: collection\n    path: src/content/projets\n    fields:\n      - name: title\n        label: Titre\n        type: string\n        required: true\n      # Adresse\n      - name: slug\n        type: string\n        pattern: ^[a-z0-9-]+$\n      - name: tags\n        type: reference\n        options: { collection: tags, multiple: true }\n\n  # ---- Tags\n  - name: tags\n    type: collection\n    path: src/content/tags\n    fields:\n      - { name: title, type: string }\n";

    fn fields(t: &str, c: usize) -> Value {
        parse(t).unwrap()["content"][c]["fields"].clone()
    }

    fn patch(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn cle_d_un_champ_modifiee_sur_place() {
        let t = edit_fields(
            CONFIG,
            "projets",
            &FieldOp::Set {
                field: "slug".into(),
                patch: patch(json!({ "label": "Adresse", "description": "Ne plus modifier : référencement" })),
            },
        )
        .unwrap();
        assert_eq!(
            t,
            CONFIG.replace(
                "        pattern: ^[a-z0-9-]+$\n",
                "        pattern: ^[a-z0-9-]+$\n        label: Adresse\n        description: \"Ne plus modifier : référencement\"\n"
            )
        );
        // Clé retirée, clé sur la ligne du tiret modifiée : le reste du texte est gardé.
        let t2 = edit_fields(
            &t,
            "projets",
            &FieldOp::Set {
                field: "title".into(),
                patch: patch(json!({ "required": null, "label": "Titre du projet" })),
            },
        )
        .unwrap();
        assert!(t2.contains("      - name: title\n        label: Titre du projet\n        type: string\n      # Adresse\n"));
        assert!(t2.starts_with("# Configuration\nmedia:"));
        // Le nom ne change pas.
        assert!(
            edit_fields(
                CONFIG,
                "projets",
                &FieldOp::Set {
                    field: "slug".into(),
                    patch: patch(json!({ "name": "url" }))
                }
            )
            .is_err()
        );
    }

    #[test]
    fn champs_ajoutes_retires_deplaces() {
        let def = json!({ "name": "client", "label": "Client", "type": "string" });
        let t = edit_fields(
            CONFIG,
            "projets",
            &FieldOp::Add {
                def: def.clone(),
                index: Some(1),
            },
        )
        .unwrap();
        assert!(t.contains("        required: true\n      - name: client\n        label: Client\n        type: string\n      # Adresse\n"));
        assert_eq!(fields(&t, 0)[1], def);
        // Ajout à la fin, puis retrait : texte d'origine.
        let t = edit_fields(
            CONFIG,
            "projets",
            &FieldOp::Add {
                def: def.clone(),
                index: None,
            },
        )
        .unwrap();
        assert_eq!(fields(&t, 0)[3]["name"], "client");
        assert_eq!(
            edit_fields(
                &t,
                "projets",
                &FieldOp::Remove {
                    field: "client".into()
                }
            )
            .unwrap(),
            CONFIG
        );
        assert!(
            edit_fields(
                CONFIG,
                "projets",
                &FieldOp::Add {
                    def: json!({ "name": "slug" }),
                    index: None
                }
            )
            .is_err()
        );
        // Déplacements, aller et retour.
        let t = edit_fields(
            CONFIG,
            "projets",
            &FieldOp::Move {
                field: "title".into(),
                index: 2,
            },
        )
        .unwrap();
        let names: Vec<_> = fields(&t, 0)
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].clone())
            .collect();
        assert_eq!(names, [json!("slug"), json!("tags"), json!("title")]);
        let back = edit_fields(
            &t,
            "projets",
            &FieldOp::Move {
                field: "title".into(),
                index: 0,
            },
        )
        .unwrap();
        assert_eq!(parse(&back).unwrap(), parse(CONFIG).unwrap());
        // Liste en style compact : réécrite en bloc (collection « tags »).
        let t = edit_fields(
            CONFIG,
            "tags",
            &FieldOp::Set {
                field: "title".into(),
                patch: patch(json!({ "label": "Titre" })),
            },
        )
        .unwrap();
        assert_eq!(
            fields(&t, 1)[0],
            json!({ "name": "title", "type": "string", "label": "Titre" })
        );
        assert!(t.starts_with(&CONFIG[..CONFIG.find("  - name: tags").unwrap()]));
    }

    #[test]
    fn modification_fusionnee() {
        let t = edit_fields(
            CONFIG,
            "projets",
            &FieldOp::Set {
                field: "tags".into(),
                patch: patch(json!({ "options": { "multiple": null, "max": 3 } })),
            },
        )
        .unwrap();
        assert_eq!(
            fields(&t, 0)[2]["options"],
            json!({ "collection": "tags", "max": 3 })
        );
        let t = edit_fields(
            &t,
            "projets",
            &FieldOp::Set {
                field: "tags".into(),
                patch: patch(json!({ "options": { "collection": null, "max": null } })),
            },
        )
        .unwrap();
        assert!(fields(&t, 0)[2].get("options").is_none());
    }

    #[test]
    fn collections_et_reglages() {
        let t = set_collection(
            CONFIG,
            "projets",
            &patch(json!({ "view": { "fields": ["title", "slug"], "primary": null }, "label": "Réalisations" })),
        )
        .unwrap();
        let d = parse(&t).unwrap();
        assert_eq!(d["content"][0]["label"], "Réalisations");
        assert_eq!(d["content"][0]["view"]["fields"][1], "slug");
        assert!(t.contains("  # ---- Tags\n  - name: tags\n"));
        let def = json!({ "name": "offres", "type": "collection", "path": "src/content/offres", "fields": [{ "name": "title", "type": "string" }] });
        let t = add_collection(CONFIG, &def).unwrap();
        assert!(t.starts_with(CONFIG));
        assert_eq!(parse(&t).unwrap()["content"][2], def);
        assert!(add_collection(&t, &def).is_err());
        // Fichier créé, puis réglage ajouté avant `content`.
        let t = add_collection("", &def).unwrap();
        assert!(t.starts_with(HEADER));
        let t = set_top(&t, "settings", &json!({ "content": { "merge": true } })).unwrap();
        assert_eq!(parse(&t).unwrap()["settings"]["content"]["merge"], true);
        assert!(t.find("settings:").unwrap() < t.find("content:\n  - name").unwrap());
    }

    /// Toutes les opérations sur chaque champ d'un vrai fichier (`REVEL_PAGES_YML=chemin`) :
    /// chaque écriture relue, aller-retour identique au texte d'origine.
    #[test]
    #[ignore]
    fn sur_un_fichier_reel() {
        let Ok(path) = std::env::var("REVEL_PAGES_YML") else {
            return;
        };
        let text = std::fs::read_to_string(path).unwrap();
        let doc = parse(&text).unwrap();
        let mut n = 0;
        for c in doc["content"].as_array().unwrap() {
            let name = c["name"].as_str().unwrap();
            let Some(fs) = c.get("fields").and_then(Value::as_array) else {
                continue;
            };
            for (i, f) in fs.iter().enumerate() {
                let field = f["name"].as_str().unwrap().to_string();
                let t = edit_fields(
                    &text,
                    name,
                    &FieldOp::Set {
                        field: field.clone(),
                        patch: patch(json!({ "label": "Essai : libellé" })),
                    },
                )
                .unwrap();
                assert!(t.len() < text.len() + 40, "réécriture limitée au champ");
                let t = edit_fields(
                    &text,
                    name,
                    &FieldOp::Remove {
                        field: field.clone(),
                    },
                )
                .unwrap();
                let back = edit_fields(
                    &t,
                    name,
                    &FieldOp::Add {
                        def: f.clone(),
                        index: Some(i),
                    },
                )
                .unwrap();
                assert_eq!(parse(&back).unwrap(), doc);
                let last = fs.len() - 1;
                let moved = edit_fields(
                    &text,
                    name,
                    &FieldOp::Move {
                        field: field.clone(),
                        index: last,
                    },
                )
                .unwrap();
                let back = edit_fields(&moved, name, &FieldOp::Move { field, index: i }).unwrap();
                if fs.len() > 1 && c.get("fields").is_some() {
                    assert_eq!(back, text, "aller-retour {name}");
                }
                n += 1;
            }
        }
        assert!(n > 0);
    }
}
