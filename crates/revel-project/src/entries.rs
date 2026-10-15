//! Contenus de collection (étape 3.1) : champ de l'en-tête ou texte modifié, contenu créé ou
//! supprimé.
//!
//! Comme pour les valeurs liées (D-19), seule la partie visée du fichier est réécrite : un champ
//! simple garde son style d'écriture (guillemets), un champ composé (liste, objet) est réécrit en
//! bloc à sa place, un nouveau champ s'ajoute à la fin de l'en-tête ; le reste — commentaires,
//! ordre des champs, texte — est conservé octet pour octet. Le fichier reste lisible par
//! Pages CMS. Chaque écriture est relue et comparée à la valeur voulue avant d'être enregistrée.

use crate::bound::{
    BoundError, double_quoted, find_scalar, front_matter_range, plain_ok, set_yaml_value,
    write_atomic,
};
use granit_parser::ScalarStyle;
use revel_edit::canvas::Seg;
use serde_json::Value;
use std::path::PathBuf;

/// Champ de premier niveau de l'en-tête : nom et plage (octets) de ses lignes.
#[derive(Debug, Clone, PartialEq)]
struct Block {
    key: String,
    start: usize,
    end: usize,
}

/// Nom d'un champ sur une ligne de premier niveau (`titre: …`, `"a b": …`).
pub(crate) fn key_of(line: &str) -> Option<String> {
    let t = line.trim_end();
    for q in ['"', '\''] {
        if let Some(rest) = t.strip_prefix(q) {
            let end = rest.find(q)?;
            let after = &rest[end + 1..];
            return (after.starts_with(':')).then(|| rest[..end].to_string());
        }
    }
    let i = t
        .char_indices()
        .find(|&(i, c)| c == ':' && t[i + 1..].chars().next().is_none_or(char::is_whitespace))
        .map(|(i, _)| i)?;
    let k = t[..i].trim();
    (!k.is_empty()).then(|| k.to_string())
}

/// Champs de premier niveau d'un en-tête YAML. Un champ s'étend jusqu'à la ligne de premier
/// niveau suivante (champ ou commentaire) ; les lignes vides qui le terminent n'en font pas
/// partie. Les listes non indentées (`- a` au début de la ligne) appartiennent au champ.
fn blocks(yaml: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut pos = 0;
    let mut last_content_end = 0;
    for line in yaml.split_inclusive('\n') {
        let t = line.trim_end_matches(['\n', '\r']);
        let top = !t.is_empty() && !t.starts_with([' ', '\t', '-']);
        if top {
            if let Some(b) = out.last_mut()
                && b.end == usize::MAX
            {
                b.end = last_content_end;
            }
            if !t.starts_with('#')
                && let Some(key) = key_of(t)
            {
                out.push(Block {
                    key,
                    start: pos,
                    end: usize::MAX,
                });
            }
        }
        pos += line.len();
        if !t.trim().is_empty() {
            last_content_end = pos;
        }
    }
    if let Some(b) = out.last_mut()
        && b.end == usize::MAX
    {
        b.end = last_content_end.max(b.start);
    }
    out
}

/// Nom de champ écrit en YAML.
fn emit_key(k: &str) -> String {
    if !k.is_empty()
        && k.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
        && !k.starts_with('-')
    {
        k.to_string()
    } else {
        double_quoted(k)
    }
}

/// Valeur simple écrite en YAML (texte sans guillemets quand il reste un texte).
fn emit_scalar(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) if plain_ok(s, "x") || is_date(s) => s.clone(),
        Value::String(s) => double_quoted(s),
        _ => String::new(),
    }
}

/// Date ISO (`2026-10-15`, `2026-10-15T09:30:00Z`) : écrite sans guillemets, lue comme une date
/// par Eleventy et Pages CMS.
fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
        && (b.len() == 10 || (b[10] == b'T' || b[10] == b' ') && !s.contains(['#', '\n']))
}

fn is_scalar(v: &Value) -> bool {
    !matches!(v, Value::Array(_) | Value::Object(_))
}

/// Champ `key: value` écrit en YAML (style bloc, retrait de deux espaces), lignes terminées.
pub(crate) fn emit_entry(key: &str, v: &Value, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let k = emit_key(key);
    match v {
        Value::String(s) if s.contains('\n') && !s.starts_with([' ', '\t']) => {
            // Texte sur plusieurs lignes : bloc littéral.
            let chomp = if s.ends_with('\n') { "" } else { "-" };
            let mut out = format!("{pad}{k}: |{chomp}\n");
            for l in s.trim_end_matches('\n').split('\n') {
                if l.is_empty() {
                    out.push('\n');
                } else {
                    out.push_str(&format!("{pad}  {l}\n"));
                }
            }
            out
        }
        Value::Array(a) if a.is_empty() => format!("{pad}{k}: []\n"),
        Value::Object(m) if m.is_empty() => format!("{pad}{k}: {{}}\n"),
        Value::Array(a) => {
            let mut out = format!("{pad}{k}:\n");
            for item in a {
                out.push_str(&emit_item(item, indent + 2));
            }
            out
        }
        Value::Object(m) => {
            let mut out = format!("{pad}{k}:\n");
            for (k2, v2) in m {
                out.push_str(&emit_entry(k2, v2, indent + 2));
            }
            out
        }
        s => {
            let v = emit_scalar(s);
            if v.is_empty() {
                format!("{pad}{k}:\n")
            } else {
                format!("{pad}{k}: {v}\n")
            }
        }
    }
}

/// Élément de liste (`- …`) écrit en YAML.
pub(crate) fn emit_item(v: &Value, indent: usize) -> String {
    let pad = " ".repeat(indent);
    match v {
        Value::Object(m) if !m.is_empty() => {
            // Premier champ sur la ligne du tiret, les suivants alignés dessous.
            let mut out = String::new();
            for (i, (k, v2)) in m.iter().enumerate() {
                let e = emit_entry(k, v2, indent + 2);
                if i == 0 {
                    out.push_str(&format!("{pad}- {}", &e[indent + 2..]));
                } else {
                    out.push_str(&e);
                }
            }
            out
        }
        Value::Array(_) | Value::Object(_) => {
            format!("{pad}- {}\n", serde_json::to_string(v).unwrap_or_default())
        }
        Value::String(s) if s.contains('\n') => format!("{pad}- {}\n", double_quoted(s)),
        s => format!("{pad}- {}\n", emit_scalar(s)),
    }
}

/// Texte d'un nouveau contenu : en-tête (`data`, objet) puis texte.
pub fn entry_text(data: &Value, body: &str) -> String {
    let mut out = String::from("---\n");
    if let Value::Object(m) = data {
        for (k, v) in m {
            out.push_str(&emit_entry(k, v, 0));
        }
    }
    out.push_str("---\n");
    out.push_str(body);
    if !body.is_empty() && !body.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Valeur d'un champ dans un en-tête YAML (relecture).
fn read_field(yaml: &str, key: &str) -> Result<Option<Value>, BoundError> {
    let v = crate::parse_yaml(yaml)?;
    Ok(v.get(key).cloned())
}

/// Égalité de valeurs après relecture (un nombre entier peut revenir en flottant et inversement).
pub(crate) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Null, Value::String(s)) | (Value::String(s), Value::Null) => s.is_empty(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

/// Modifie (ou ajoute, ou retire avec `None`) le champ `key` de l'en-tête d'un contenu.
pub fn set_field(text: &str, key: &str, value: Option<&Value>) -> Result<String, BoundError> {
    let Some((a, z)) = front_matter_range(text) else {
        // Pas d'en-tête : créé.
        return Ok(match value {
            None => text.to_string(),
            Some(v) => format!("---\n{}---\n{text}", emit_entry(key, v, 0)),
        });
    };
    let yaml = &text[a..z];
    let found = blocks(yaml).into_iter().find(|b| b.key == key);
    let new_yaml = match (&found, value) {
        (None, None) => return Ok(text.to_string()),
        (None, Some(v)) => {
            let mut y = yaml.to_string();
            if !y.is_empty() && !y.ends_with('\n') {
                y.push('\n');
            }
            y.push_str(&emit_entry(key, v, 0));
            y
        }
        // Retrait du champ (ses lignes, retour à la ligne compris).
        (Some(b), None) => format!("{}{}", &yaml[..b.start], &yaml[b.end..]),
        (Some(b), Some(v)) => {
            let in_place = is_scalar(v)
                .then(|| {
                    find_scalar(yaml, &[Seg::Key(key.to_string())])
                        .ok()
                        .flatten()
                })
                .flatten()
                .filter(|s| s.start >= b.start && s.end <= b.end);
            match (in_place, v) {
                // Texte sur une ligne : style d'écriture gardé (sans guillemets, guillemets) ; sur
                // plusieurs lignes : bloc littéral réécrit à sa place (plus bas).
                (Some(_), Value::String(s)) if !s.contains('\n') => {
                    set_yaml_value(yaml, &[Seg::Key(key.to_string())], s)?.0
                }
                (Some(s), _)
                    if !matches!(s.style, ScalarStyle::Literal | ScalarStyle::Folded)
                        && !matches!(v, Value::String(_))
                        && !emit_scalar(v).is_empty() =>
                {
                    format!("{}{}{}", &yaml[..s.start], emit_scalar(v), &yaml[s.end..])
                }
                // Réécrit en bloc à sa place ; une liste garde le retrait de ses tirets (`- `
                // au début de la ligne, ou décalé de deux espaces).
                _ => {
                    let old = &yaml[b.start..b.end];
                    let dash = old
                        .lines()
                        .nth(1)
                        .filter(|l| l.trim_start().starts_with('-'))
                        .map(|l| l.len() - l.trim_start().len());
                    let entry = match (v, dash) {
                        (Value::Array(a), Some(d)) if !a.is_empty() => {
                            let mut out = format!("{}:\n", emit_key(key));
                            for item in a {
                                out.push_str(&emit_item(item, d));
                            }
                            out
                        }
                        _ => emit_entry(key, v, 0),
                    };
                    format!("{}{}{}", &yaml[..b.start], entry, &yaml[b.end..])
                }
            }
        }
    };
    // Relecture : la valeur lue doit être celle voulue, les autres champs inchangés.
    let before = crate::parse_yaml(yaml)?;
    let after = crate::parse_yaml(&new_yaml).map_err(|e| format!("écriture refusée : {e}"))?;
    let got = read_field(&new_yaml, key)?;
    let ok_value = match (value, &got) {
        (None, None) => true,
        (Some(v), Some(g)) => same(v, g),
        (Some(Value::Null), None) => true,
        _ => false,
    };
    let others_kept = match (&before, &after) {
        (Value::Object(x), Value::Object(y)) => x
            .iter()
            .filter(|(k, _)| k.as_str() != key)
            .all(|(k, v)| y.get(k).is_some_and(|w| same(v, w))),
        _ => false,
    };
    if !ok_value || !others_kept {
        return Err("écriture refusée : relecture différente".into());
    }
    Ok(format!("{}{new_yaml}{}", &text[..a], &text[z..]))
}

/// Remplace le texte (Markdown) d'un contenu, après son en-tête.
pub fn set_body(text: &str, body: &str) -> String {
    let mut body = body.to_string();
    if !body.is_empty() && !body.ends_with('\n') {
        body.push('\n');
    }
    match front_matter_range(text) {
        Some((_, z)) => {
            // Fin de la ligne `---` qui ferme l'en-tête.
            let close = text[z..].find('\n').map_or(text.len(), |i| z + i + 1);
            let head = &text[..close];
            if head.ends_with('\n') {
                format!("{head}{body}")
            } else {
                format!("{head}\n{body}")
            }
        }
        None => body,
    }
}

/// Nom de fichier accepté pour un contenu (sans extension).
fn valid_stem(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('.')
        && s.len() <= 120
        && s.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl crate::Project {
    /// Fichier d'un contenu : dossier d'une collection du site et nom sans extension. Fichier
    /// existant (`.md` ou `.markdown`), sinon `<dossier>/<nom>.md`.
    pub fn entry_path(&self, folder: &str, file: &str) -> Result<PathBuf, BoundError> {
        if !self
            .site
            .collections
            .iter()
            .any(|c| !c.folder.is_empty() && c.folder == folder)
        {
            return Err(format!("collection inconnue : {folder}"));
        }
        if !valid_stem(file) {
            return Err(format!("nom de fichier refusé : {file}"));
        }
        let base = self.dir.join(folder);
        Ok(["md", "markdown"]
            .iter()
            .map(|e| base.join(format!("{file}.{e}")))
            .find(|p| p.is_file())
            .unwrap_or_else(|| base.join(format!("{file}.md"))))
    }

    /// Texte actuel d'un contenu (`None` s'il n'existe pas).
    pub fn entry_text(&self, folder: &str, file: &str) -> Result<Option<String>, BoundError> {
        let p = self.entry_path(folder, file)?;
        if !p.is_file() {
            return Ok(None);
        }
        std::fs::read_to_string(&p)
            .map(Some)
            .map_err(|e| format!("{} : {e}", p.display()))
    }

    /// Remplace le fichier d'un contenu par `text` (`None` : supprimé) ; renvoie l'ancien texte.
    pub fn write_entry(
        &self,
        folder: &str,
        file: &str,
        text: Option<&str>,
    ) -> Result<Option<String>, BoundError> {
        let p = self.entry_path(folder, file)?;
        let old = self.entry_text(folder, file)?;
        match text {
            Some(t) => {
                if let Some(dir) = p.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
                }
                write_atomic(&p, t)?;
            }
            None => {
                if p.is_file() {
                    std::fs::remove_file(&p).map_err(|e| format!("{} : {e}", p.display()))?;
                    // Dossier resté vide (créé pour ce contenu) : retiré, comme le ferait Git.
                    if let Some(dir) = p.parent()
                        && std::fs::read_dir(dir).is_ok_and(|mut d| d.next().is_none())
                    {
                        let _ = std::fs::remove_dir(dir);
                    }
                }
            }
        }
        Ok(old)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DOC: &str = "---\n# Projet\ntitle: 'Wattr'\nclient: Wattr\ndate: 2025-03-12\ntags:\n  - film\n  - ia\nfeatured: true\n\nlayout: projet.njk\n---\nTexte du projet.\n";

    fn field(text: &str, key: &str) -> Option<Value> {
        let (a, z) = front_matter_range(text).unwrap();
        crate::parse_yaml(&text[a..z]).unwrap().get(key).cloned()
    }

    #[test]
    fn champ_simple_modifie_sur_place() {
        let t = set_field(DOC, "title", Some(&json!("L'atelier"))).unwrap();
        assert!(t.contains("title: 'L''atelier'\n"), "{t}");
        assert_eq!(t.replace("title: 'L''atelier'", "title: 'Wattr'"), DOC);
        let t = set_field(DOC, "featured", Some(&json!(false))).unwrap();
        assert_eq!(t.replace("featured: false", "featured: true"), DOC);
        let t = set_field(DOC, "date", Some(&json!("2025-04-01"))).unwrap();
        assert_eq!(field(&t, "date"), Some(json!("2025-04-01")));
        assert!(t.contains("date: 2025-04-01\n"));
    }

    #[test]
    fn liste_reecrite_en_bloc_a_sa_place() {
        let t = set_field(DOC, "tags", Some(&json!(["film", "motion", "3d"]))).unwrap();
        assert!(
            t.contains("tags:\n  - film\n  - motion\n  - 3d\nfeatured: true\n"),
            "{t}"
        );
        assert_eq!(field(&t, "tags"), Some(json!(["film", "motion", "3d"])));
        assert!(t.ends_with("layout: projet.njk\n---\nTexte du projet.\n"));
    }

    #[test]
    fn champ_ajoute_puis_retire() {
        let t = set_field(
            DOC,
            "credits",
            Some(&json!([{ "role": "Réalisation", "nom": "A. B." }])),
        )
        .unwrap();
        assert_eq!(
            field(&t, "credits"),
            Some(json!([{ "role": "Réalisation", "nom": "A. B." }]))
        );
        let back = set_field(&t, "credits", None).unwrap();
        assert_eq!(back, DOC);
        let t = set_field(DOC, "client", None).unwrap();
        assert!(!t.contains("client:"));
        assert_eq!(field(&t, "title"), Some(json!("Wattr")));
        let t = set_field(DOC, "resume", Some(&json!("Ligne 1\nLigne 2"))).unwrap();
        assert_eq!(field(&t, "resume"), Some(json!("Ligne 1\nLigne 2")));
    }

    #[test]
    fn texte_sur_plusieurs_lignes_en_bloc() {
        let doc = "---\nintro: |-\n  <p>Un</p>\n  <p>Deux</p>\nfin: 1\n---\n";
        let t = set_field(doc, "intro", Some(&json!("<p>Un</p>\n<p>Trois</p>"))).unwrap();
        assert_eq!(
            t,
            "---\nintro: |-\n  <p>Un</p>\n  <p>Trois</p>\nfin: 1\n---\n"
        );
        let t = set_field(doc, "intro", Some(&json!("<p>Seul</p>"))).unwrap();
        assert_eq!(field(&t, "intro"), Some(json!("<p>Seul</p>")));
        assert!(t.ends_with("fin: 1\n---\n"));
    }

    #[test]
    fn texte_et_nouveau_contenu() {
        assert_eq!(
            set_body(DOC, "Autre texte."),
            DOC.replace("Texte du projet.\n", "Autre texte.\n")
        );
        let t = entry_text(
            &json!({ "title": "Nouveau : essai", "date": "2026-10-15", "draft": true }),
            "",
        );
        assert_eq!(
            t,
            "---\ntitle: \"Nouveau : essai\"\ndate: 2026-10-15\ndraft: true\n---\n"
        );
        assert_eq!(field(&t, "title"), Some(json!("Nouveau : essai")));
        assert_eq!(set_body(&t, "Bonjour"), format!("{t}Bonjour\n"));
    }

    #[test]
    fn liste_reecrite_avec_son_retrait() {
        let text = "---\ntitle: A\nblocks:\n- type: quote\n  quote: |-\n    <p>Q</p>\n- type: text\n  body: x\ndate: 2026\n---\nCorps\n";
        let v: Value = serde_json::json!([
            { "type": "text", "body": "x" },
            { "type": "quote", "quote": "<p>Q</p>" }
        ]);
        let out = set_field(text, "blocks", Some(&v)).unwrap();
        assert_eq!(
            out,
            "---\ntitle: A\nblocks:\n- type: text\n  body: x\n- type: quote\n  quote: <p>Q</p>\ndate: 2026\n---\nCorps\n"
        );
    }
}
