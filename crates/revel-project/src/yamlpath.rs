//! Valeur d'un fichier YAML (fichier de données) écrite à un chemin de clés, le reste du texte
//! inchangé (commentaires, ordre, style) : la clé existante est réécrite à sa place, une clé
//! absente est ajoutée à la fin de son objet (les objets intermédiaires sont créés), `None` la
//! retire. Seul le style bloc est pris en charge (objets `{…}` sur une ligne refusés) ; le
//! résultat est relu et doit donner exactement le document attendu.

use crate::bound::BoundError;
use crate::entries::emit_entry;
use serde_json::{Map, Value};

/// Ligne significative : indentation, clé (`clé:` en début de ligne), position.
struct Line {
    start: usize,
    end: usize,
    indent: usize,
    key: Option<String>,
    blank: bool,
    /// Élément de liste (`- …`).
    dash: bool,
}

fn lines(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    let mut pos = 0;
    for l in text.split_inclusive('\n') {
        let body = l.trim_end_matches(['\n', '\r']);
        let trimmed = body.trim_start();
        let indent = body.len() - trimmed.len();
        let blank = trimmed.is_empty() || trimmed.starts_with('#');
        out.push(Line {
            start: pos,
            end: pos + l.len(),
            indent,
            key: if blank {
                None
            } else {
                crate::entries::key_of(trimmed)
            },
            blank,
            dash: !blank && (trimmed == "-" || trimmed.starts_with("- ")),
        });
        pos += l.len();
    }
    out
}

/// Fin (exclue) du bloc de la ligne `i` : première ligne significative suivante dont
/// l'indentation est au plus la sienne (lignes vides et commentaires de fin exclus). Les
/// éléments d'une liste écrite au même retrait que sa clé (`clé:` puis `- a`) en font partie.
fn block_end(ls: &[Line], i: usize) -> usize {
    let ind = ls[i].indent;
    let mut j = i + 1;
    let mut last = i + 1;
    while j < ls.len() {
        if !ls[j].blank {
            if ls[j].indent < ind || (ls[j].indent == ind && !(ls[j].dash && !ls[i].dash)) {
                break;
            }
            last = j + 1;
        }
        j += 1;
    }
    last
}

/// Clé `key` parmi les enfants directs de la plage `[a, b)` (indentation `ind`).
fn find(ls: &[Line], a: usize, b: usize, ind: usize, key: &str) -> Option<usize> {
    (a..b).find(|&i| !ls[i].blank && ls[i].indent == ind && ls[i].key.as_deref() == Some(key))
}

/// Indentation des enfants de la ligne `i` (bloc non vide), sinon `None`.
fn child_indent(ls: &[Line], i: usize, end: usize) -> Option<usize> {
    (i + 1..end).find(|&j| !ls[j].blank).map(|j| ls[j].indent)
}

/// Valeur à écrire pour `path` (objets intermédiaires).
fn nest(path: &[String], v: &Value) -> Value {
    match path.split_first() {
        None => v.clone(),
        Some((k, rest)) => {
            let mut m = Map::new();
            m.insert(k.clone(), nest(rest, v));
            Value::Object(m)
        }
    }
}

/// Document attendu : `doc` avec la valeur au chemin (créée, remplacée ou retirée).
fn expected(doc: &Value, path: &[String], v: Option<&Value>) -> Option<Value> {
    let mut d = doc.clone();
    let mut cur = &mut d;
    for (i, k) in path.iter().enumerate() {
        if cur.is_null() {
            *cur = Value::Object(Map::new());
        }
        let m = cur.as_object_mut()?;
        if i + 1 == path.len() {
            match v {
                Some(v) => {
                    m.insert(k.clone(), v.clone());
                }
                None => {
                    m.remove(k);
                }
            }
            return Some(d);
        }
        if !m.contains_key(k) {
            if v.is_none() {
                return Some(d);
            }
            m.insert(k.clone(), Value::Object(Map::new()));
        }
        cur = m.get_mut(k)?;
    }
    Some(d)
}

/// Écrit `value` au chemin `path` (clés d'objets) dans le texte YAML `text`.
pub fn set_path(text: &str, path: &[String], value: Option<&Value>) -> Result<String, BoundError> {
    if path.is_empty() {
        return Err("chemin vide".into());
    }
    let doc = if text.trim().is_empty() {
        Value::Object(Map::new())
    } else {
        crate::parse_yaml(text).map_err(|e| format!("fichier illisible : {e}"))?
    };
    let want = expected(&doc, path, value).ok_or("valeur à ce chemin qui n'est pas un objet")?;
    if want == doc {
        return Ok(text.to_string());
    }
    // Texte à la place d'un texte : réécrit dans son style (guillemets, bloc, commentaire gardé).
    if let Some(Value::String(s)) = value {
        let segs: Vec<revel_edit::canvas::Seg> = path
            .iter()
            .map(|k| revel_edit::canvas::Seg::Key(k.clone()))
            .collect();
        if let Ok((t, _)) = crate::bound::set_yaml_value(text, &segs, s)
            && crate::parse_yaml(&t).ok().as_ref() == Some(&want)
        {
            return Ok(t);
        }
    }
    let ls = lines(text);
    let (mut a, mut b, mut ind) = (0usize, ls.len(), 0usize);
    let mut out: Option<String> = None;
    for (depth, key) in path.iter().enumerate() {
        let last = depth + 1 == path.len();
        match find(&ls, a, b, ind, key) {
            Some(i) => {
                let end = block_end(&ls, i);
                if last {
                    let piece = value.map(|v| emit_entry(key, v, ind)).unwrap_or_default();
                    out = Some(format!(
                        "{}{piece}{}",
                        &text[..ls[i].start],
                        &text[ls.get(end).map_or(text.len(), |l| l.start)..]
                    ));
                    break;
                }
                match child_indent(&ls, i, end) {
                    Some(ci) => {
                        (a, b, ind) = (i + 1, end, ci);
                    }
                    None => {
                        // Objet vide ou valeur simple : la clé est réécrite avec le reste du
                        // chemin.
                        let Some(v) = value else {
                            return Ok(text.to_string());
                        };
                        let piece = emit_entry(key, &nest(&path[depth + 1..], v), ind);
                        out = Some(format!(
                            "{}{piece}{}",
                            &text[..ls[i].start],
                            &text[ls.get(end).map_or(text.len(), |l| l.start)..]
                        ));
                        break;
                    }
                }
            }
            None => {
                let Some(v) = value else {
                    return Ok(text.to_string());
                };
                // Ajout à la fin de l'objet : après sa dernière ligne significative.
                let at = (a..b)
                    .rev()
                    .find(|&j| !ls[j].blank)
                    .map(|j| ls[j].end)
                    .unwrap_or(if a < ls.len() {
                        ls[a].start
                    } else {
                        text.len()
                    });
                let piece = emit_entry(key, &nest(&path[depth + 1..], v), ind);
                let sep = if at > 0 && !text[..at].ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                out = Some(format!("{}{sep}{piece}{}", &text[..at], &text[at..]));
                break;
            }
        }
    }
    let new = out.ok_or("chemin introuvable")?;
    let check = crate::parse_yaml(&new).map_err(|e| format!("écriture illisible : {e}"))?;
    if check != want {
        return Err(
            "écriture refusée : relecture différente (format du fichier non pris en charge)".into(),
        );
    }
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn p(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn chemins_ecrits_sur_place() {
        let src = "# Page agence\nhero:\n  title: L'agence   # titre\nseo:\n  title: Agence\n  description: >-\n    Une agence\n    plurielle\n\nfooter: oui\n";
        // Valeur existante : seule sa ligne change (commentaire gardé).
        let t = set_path(src, &p(&["seo", "title"]), Some(&json!("Notre agence"))).unwrap();
        assert_eq!(
            t,
            src.replace("  title: Agence\n", "  title: Notre agence\n")
        );
        let t = set_path(src, &p(&["hero", "title"]), Some(&json!("L'atelier"))).unwrap();
        assert_eq!(t, src.replace("L'agence   # titre", "L'atelier   # titre"));
        // Clé absente : ajoutée à la fin de son objet.
        let t = set_path(src, &p(&["seo", "image"]), Some(&json!("/media/a.jpg"))).unwrap();
        assert!(
            t.contains("    plurielle\n  image: /media/a.jpg\n\nfooter: oui\n"),
            "{t}"
        );
        // Objet intermédiaire créé.
        let t = set_path(src, &p(&["social", "x"]), Some(&json!("@alunites"))).unwrap();
        assert!(
            t.ends_with("footer: oui\nsocial:\n  x: \"@alunites\"\n"),
            "{t}"
        );
        // Retrait (texte sur plusieurs lignes compris).
        let t = set_path(src, &p(&["seo", "description"]), None).unwrap();
        assert_eq!(
            t,
            "# Page agence\nhero:\n  title: L'agence   # titre\nseo:\n  title: Agence\n\nfooter: oui\n"
        );
        // Rien à retirer : texte inchangé.
        assert_eq!(set_path(src, &p(&["seo", "absent"]), None).unwrap(), src);
        // Objet vide écrit `{}` : réécrit.
        let t = set_path("seo: {}\n", &p(&["seo", "title"]), Some(&json!("A"))).unwrap();
        assert_eq!(t, "seo:\n  title: A\n");
        // Fichier vide.
        assert_eq!(set_path("", &p(&["a"]), Some(&json!(1))).unwrap(), "a: 1\n");
        // Valeur simple à la place d'un objet : refusé.
        assert!(set_path("seo: x\n", &p(&["seo", "title"]), Some(&json!("A"))).is_err());
        // Liste écrite au même retrait que sa clé (style de Pages CMS) : remplacée entière.
        let src = "seo:\n  title: A\nfeatured:\n- un\n- deux\nnews:\n- trois\n";
        let t = set_path(src, &p(&["featured"]), Some(&json!(["deux", "quatre"]))).unwrap();
        assert_eq!(
            crate::parse_yaml(&t).unwrap(),
            json!({ "seo": { "title": "A" }, "featured": ["deux", "quatre"], "news": ["trois"] })
        );
        assert!(t.starts_with("seo:\n  title: A\nfeatured:\n"), "{t}");
        assert!(t.ends_with("news:\n- trois\n"), "{t}");
    }
}
