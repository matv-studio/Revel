//! Fusion à trois du document de site (JSON) quand deux personnes l'ont modifié (D-89) : les
//! modifications faites à des endroits différents se combinent (pages, composants, éléments,
//! règles de style…) ; il ne reste à choisir que les endroits modifiés des deux côtés.
//!
//! - Objets : fusion clé par clé.
//! - Listes d'objets portant un `id` (pages, gabarits, composants, éléments, avec les textes
//!   placés entre eux) : fusion par identifiant (ajouts et retraits de chaque côté gardés, ordre
//!   de la version locale).
//! - Autres listes (règles de style…) : fusion par plages, comme Git pour les lignes d'un texte ;
//!   ajouts des deux côtés au même endroit gardés tous les deux ; plage modifiée des deux côtés :
//!   élément par élément si sa longueur n'a pas changé, sinon point à choisir.
//! - Valeurs simples modifiées des deux côtés : point à choisir.

use serde_json::{Map, Value};

/// Étape d'un chemin dans le document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    Key(String),
    /// Élément d'une liste repéré par son `id`.
    Id(String),
    Index(usize),
    /// Plage `début..fin` d'une liste de la version d'origine.
    Range(usize, usize),
}

/// Chemin écrit (`/pages/@index/body/@a1b2/attrs/#0`) : identifiant d'un point à choisir.
pub fn path_id(path: &[Seg]) -> String {
    let mut s = String::new();
    for p in path {
        match p {
            Seg::Key(k) => {
                s.push('/');
                s.push_str(k);
            }
            Seg::Id(i) => {
                s.push_str("/@");
                s.push_str(i);
            }
            Seg::Index(i) => s.push_str(&format!("/#{i}")),
            Seg::Range(a, b) => s.push_str(&format!("/#{a}-{b}")),
        }
    }
    s
}

/// Endroit modifié des deux côtés.
#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub path: Vec<Seg>,
    pub base: Option<Value>,
    /// Version locale (`None` : retirée).
    pub ours: Option<Value>,
    /// Version récupérée (`None` : retirée).
    pub theirs: Option<Value>,
}

/// Fusionne `ours` et `theirs`, issus de `base`. `choice(chemin)` : `Some(true)` garde la version
/// locale, `Some(false)` la version récupérée ; `None` : point à choisir, version locale gardée
/// en attendant.
pub fn merge3(
    base: &Value,
    ours: &Value,
    theirs: &Value,
    choice: &dyn Fn(&str) -> Option<bool>,
) -> (Value, Vec<Conflict>) {
    let mut conflicts = Vec::new();
    let mut path = Vec::new();
    let v = merge(
        Some(base),
        Some(ours),
        Some(theirs),
        &mut path,
        choice,
        &mut conflicts,
    )
    .unwrap_or(Value::Null);
    (v, conflicts)
}

fn id_of(v: &Value) -> Option<&str> {
    v.get("id").and_then(Value::as_str)
}

fn find<'a>(l: &[(String, &'a Value)], id: &str) -> Option<&'a Value> {
    l.iter().find(|(i, _)| i == id).map(|(_, v)| *v)
}

/// Éléments d'une liste repérés par leur `id` ; ceux qui n'en ont pas (textes entre deux
/// éléments) par l'`id` qui les précède et leur rang (`h1+1`). `None` : `id` en double.
fn keyed(list: &[Value]) -> Option<Vec<(String, &Value)>> {
    let mut out: Vec<(String, &Value)> = Vec::with_capacity(list.len());
    let (mut prev, mut n) = ("^".to_string(), 0);
    for v in list {
        let key = match id_of(v) {
            Some(id) => {
                prev = id.to_string();
                n = 0;
                id.to_string()
            }
            None => {
                n += 1;
                format!("{prev}+{n}")
            }
        };
        if out.iter().any(|(k, _)| *k == key) {
            return None;
        }
        out.push((key, v));
    }
    Some(out)
}

/// Paires d'éléments égaux des deux listes, dans l'ordre (plus longue sous-suite commune).
/// `None` : listes trop longues pour être comparées.
fn common(a: &[Value], b: &[Value]) -> Option<Vec<(usize, usize)>> {
    let (n, m) = (a.len(), b.len());
    if n.saturating_mul(m) > 4_000_000 {
        return None;
    }
    let mut t = vec![0u32; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[at(i, j)] = if a[i] == b[j] {
                t[at(i + 1, j + 1)] + 1
            } else {
                t[at(i + 1, j)].max(t[at(i, j + 1)])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n && j < m {
        if a[i] == b[j] {
            out.push((i, j));
            i += 1;
            j += 1;
        } else if t[at(i + 1, j)] >= t[at(i, j + 1)] {
            i += 1;
        } else {
            j += 1;
        }
    }
    Some(out)
}

/// Fusion par plages d'une liste sans identifiants : plages inchangées des deux côtés comme
/// repères, plages entre deux repères fusionnées.
fn merge_ranges(
    b: &[Value],
    o: &[Value],
    t: &[Value],
    path: &mut Vec<Seg>,
    choice: &dyn Fn(&str) -> Option<bool>,
    conflicts: &mut Vec<Conflict>,
) -> Option<Vec<Value>> {
    let mo: std::collections::HashMap<usize, usize> = common(b, o)?.into_iter().collect();
    let mt: std::collections::HashMap<usize, usize> = common(b, t)?.into_iter().collect();
    let mut out = Vec::new();
    let (mut i, mut j, mut k) = (0, 0, 0);
    loop {
        let anchor = (i..b.len()).find_map(|x| match (mo.get(&x), mt.get(&x)) {
            (Some(&oj), Some(&tk)) if oj >= j && tk >= k => Some((x, oj, tk)),
            _ => None,
        });
        let (bx, oj, tk) = anchor.unwrap_or((b.len(), o.len(), t.len()));
        let (bc, oc, tc) = (&b[i..bx], &o[j..oj], &t[k..tk]);
        if oc == tc || tc == bc {
            out.extend_from_slice(oc);
        } else if oc == bc {
            out.extend_from_slice(tc);
        } else if bc.is_empty() {
            // Ajouts des deux côtés au même endroit : tous gardés.
            out.extend_from_slice(oc);
            out.extend_from_slice(tc);
        } else if oc.len() == bc.len() && tc.len() == bc.len() {
            for n in 0..bc.len() {
                path.push(Seg::Index(i + n));
                if let Some(v) = merge(
                    Some(&bc[n]),
                    Some(&oc[n]),
                    Some(&tc[n]),
                    path,
                    choice,
                    conflicts,
                ) {
                    out.push(v);
                }
                path.pop();
            }
        } else {
            path.push(Seg::Range(i, bx));
            let arr = |s: &[Value]| Value::Array(s.to_vec());
            let picked = resolve(
                Some(&arr(bc)),
                Some(&arr(oc)),
                Some(&arr(tc)),
                path,
                choice,
                conflicts,
            );
            path.pop();
            if let Some(Value::Array(v)) = picked {
                out.extend(v);
            }
        }
        if anchor.is_none() {
            return Some(out);
        }
        out.push(b[bx].clone());
        (i, j, k) = (bx + 1, oj + 1, tk + 1);
    }
}

fn resolve(
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
    path: &[Seg],
    choice: &dyn Fn(&str) -> Option<bool>,
    conflicts: &mut Vec<Conflict>,
) -> Option<Value> {
    match choice(&path_id(path)) {
        Some(true) => ours.cloned(),
        Some(false) => theirs.cloned(),
        None => {
            conflicts.push(Conflict {
                path: path.to_vec(),
                base: base.cloned(),
                ours: ours.cloned(),
                theirs: theirs.cloned(),
            });
            ours.cloned()
        }
    }
}

fn merge(
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
    path: &mut Vec<Seg>,
    choice: &dyn Fn(&str) -> Option<bool>,
    conflicts: &mut Vec<Conflict>,
) -> Option<Value> {
    if ours == theirs {
        return ours.cloned();
    }
    if ours == base {
        return theirs.cloned();
    }
    if theirs == base {
        return ours.cloned();
    }
    match (base, ours, theirs) {
        (b, Some(Value::Object(o)), Some(Value::Object(t))) => {
            let empty = Map::new();
            let b = b.and_then(Value::as_object).unwrap_or(&empty);
            let mut out = Map::new();
            let mut keys: Vec<&String> = o.keys().collect();
            for k in t.keys() {
                if !o.contains_key(k) {
                    keys.push(k);
                }
            }
            // Clé retirée d'un côté, absente de l'autre ou inchangée : retirée.
            for k in keys {
                path.push(Seg::Key(k.clone()));
                if let Some(v) = merge(b.get(k), o.get(k), t.get(k), path, choice, conflicts) {
                    out.insert(k.clone(), v);
                }
                path.pop();
            }
            Some(Value::Object(out))
        }
        (b, Some(Value::Array(o)), Some(Value::Array(t))) => {
            let b_list: &[Value] = b.and_then(Value::as_array).map_or(&[], |v| v.as_slice());
            let has_id = [o, t, b_list]
                .iter()
                .any(|l| l.iter().any(|v| id_of(v).is_some()));
            if let (true, Some(oi), Some(ti), Some(bi)) =
                (has_id, keyed(o), keyed(t), keyed(b_list))
            {
                let mut out = Vec::new();
                // Ordre de la version locale ; éléments ajoutés de l'autre côté insérés après
                // leur prédécesseur.
                let mut order: Vec<&str> = oi.iter().map(|(i, _)| i.as_str()).collect();
                for (n, (id, _)) in ti.iter().enumerate() {
                    let id = id.as_str();
                    if order.contains(&id) || find(&bi, id).is_some() {
                        continue;
                    }
                    let after = ti[..n]
                        .iter()
                        .rev()
                        .find_map(|(p, _)| order.iter().position(|x| *x == p.as_str()));
                    match after {
                        Some(pos) => order.insert(pos + 1, id),
                        None => order.insert(0, id),
                    }
                }
                // Éléments retirés par la version locale mais présents ailleurs.
                for (id, _) in &ti {
                    if !order.contains(&id.as_str()) {
                        order.push(id);
                    }
                }
                for id in order {
                    path.push(Seg::Id(id.to_string()));
                    if let Some(v) = merge(
                        find(&bi, id),
                        find(&oi, id),
                        find(&ti, id),
                        path,
                        choice,
                        conflicts,
                    ) {
                        out.push(v);
                    }
                    path.pop();
                }
                return Some(Value::Array(out));
            }
            if let Some(out) = merge_ranges(b_list, o, t, path, choice, conflicts) {
                return Some(Value::Array(out));
            }
            resolve(base, ours, theirs, path, choice, conflicts)
        }
        _ => resolve(base, ours, theirs, path, choice, conflicts),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn site() -> Value {
        json!({
            "name": "Atelier",
            "pages": [
                {"id": "index", "path": "/", "body": [
                    {"node": "element", "id": "h1", "tag": "h1", "children": [{"node": "text", "text": "Bonjour"}]},
                    {"node": "text", "text": "\n"},
                    {"node": "element", "id": "p1", "tag": "p", "children": [{"node": "text", "text": "Texte"}]}
                ]},
                {"id": "contact", "path": "/contact/", "body": []}
            ],
            "stylesheet": {"items": [{"item": "raw", "css": "a{}"}, {"item": "raw", "css": "b{}"}]}
        })
    }

    #[test]
    fn modifications_a_des_endroits_differents() {
        let base = site();
        let mut ours = site();
        let mut theirs = site();
        // Ici : titre de l'accueil et première règle ; là-bas : paragraphe, page ajoutée, seconde règle.
        ours["pages"][0]["body"][0]["children"][0]["text"] = json!("Salut");
        ours["stylesheet"]["items"][0]["css"] = json!("a{color:red}");
        theirs["pages"][0]["body"][2]["children"][0]["text"] = json!("Autre texte");
        theirs["pages"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "blog", "path": "/blog/", "body": []}));
        theirs["stylesheet"]["items"][1]["css"] = json!("b{color:blue}");
        let (m, c) = merge3(&base, &ours, &theirs, &|_| None);
        assert!(c.is_empty(), "{c:?}");
        assert_eq!(m["pages"][0]["body"][0]["children"][0]["text"], "Salut");
        assert_eq!(
            m["pages"][0]["body"][2]["children"][0]["text"],
            "Autre texte"
        );
        assert_eq!(m["pages"].as_array().unwrap().len(), 3);
        assert_eq!(m["pages"][2]["id"], "blog");
        assert_eq!(m["stylesheet"]["items"][0]["css"], "a{color:red}");
        assert_eq!(m["stylesheet"]["items"][1]["css"], "b{color:blue}");
    }

    #[test]
    fn meme_endroit_point_a_choisir() {
        let base = site();
        let mut ours = site();
        let mut theirs = site();
        ours["pages"][0]["body"][0]["children"][0]["text"] = json!("Salut");
        theirs["pages"][0]["body"][0]["children"][0]["text"] = json!("Hello");
        // Page retirée d'un côté, modifiée de l'autre.
        ours["pages"].as_array_mut().unwrap().remove(1);
        theirs["pages"][1]["body"] = json!([{"node": "text", "text": "x"}]);
        let (m, c) = merge3(&base, &ours, &theirs, &|_| None);
        let ids: Vec<String> = c.iter().map(|c| path_id(&c.path)).collect();
        assert_eq!(
            ids,
            vec!["/pages/@index/body/@h1/children/#0/text", "/pages/@contact"]
        );
        // En attendant le choix : version locale.
        assert_eq!(m["pages"][0]["body"][0]["children"][0]["text"], "Salut");
        assert_eq!(m["pages"].as_array().unwrap().len(), 1);
        // Choix : texte récupéré, page gardée.
        let (m, c) = merge3(&base, &ours, &theirs, &|_| Some(false));
        assert!(c.is_empty());
        assert_eq!(m["pages"][0]["body"][0]["children"][0]["text"], "Hello");
        assert_eq!(m["pages"][1]["id"], "contact");
    }

    #[test]
    fn listes_sans_identifiants() {
        // Plages différentes modifiées : fusion (élément retiré ici, ajouté là-bas).
        let base = json!({"items": [1, 2]});
        let (m, c) = merge3(
            &base,
            &json!({"items": [1, 2, 3]}),
            &json!({"items": [0, 2]}),
            &|_| None,
        );
        assert!(c.is_empty(), "{c:?}");
        assert_eq!(m, json!({"items": [0, 2, 3]}));
        // Ajouts des deux côtés au même endroit : tous gardés.
        let (m, c) = merge3(&json!([1]), &json!([1, 2]), &json!([1, 3]), &|_| None);
        assert!(c.is_empty());
        assert_eq!(m, json!([1, 2, 3]));
        // Même plage changée des deux côtés : point à choisir.
        let base = json!({"items": [1, 2, 3]});
        let (ours, theirs) = (json!({"items": [1, 9, 9, 3]}), json!({"items": [1, 8, 3]}));
        let (m, c) = merge3(&base, &ours, &theirs, &|_| None);
        assert_eq!(c.len(), 1);
        assert_eq!(path_id(&c[0].path), "/items/#1-2");
        assert_eq!(m, ours);
        let (m, _) = merge3(&base, &ours, &theirs, &|_| Some(false));
        assert_eq!(m, theirs);
        // Textes entre les éléments repérés par l'élément qui les précède.
        let base = json!([{"id": "a"}, {"text": " "}, {"id": "b"}]);
        let ours = json!([{"id": "a"}, {"text": "  "}, {"id": "b"}]);
        let theirs = json!([{"id": "a"}, {"text": " "}, {"id": "b", "x": 1}, {"id": "c"}]);
        let (m, c) = merge3(&base, &ours, &theirs, &|_| None);
        assert!(c.is_empty());
        assert_eq!(
            m,
            json!([{"id": "a"}, {"text": "  "}, {"id": "b", "x": 1}, {"id": "c"}])
        );
    }
}
