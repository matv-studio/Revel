//! Liste de collection (3.7) : la boucle d'une liste de contenus, réglée sans écrire
//! d'expression (comme la « Collection List » de Webflow) : collection, filtre, tri, nombre.
//!
//! La boucle porte une expression lisible et modifiable à la main :
//! `(collections.projets | selectattr("data.une") | sort(attribute="date", reverse=true))[:3]`.
//! [`Query::parse`] relit seulement cette forme ; une autre expression reste une liste calculée.

use serde::{Deserialize, Serialize};

/// Réglages d'une liste de collection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    /// Identifiant de la collection (`collections.<id>`).
    pub collection: String,
    /// Filtre : chemin dans le contenu (`data.categorie`) et valeur attendue (expression
    /// littérale : `"design"`, `true`, `3`) ; sans valeur, le champ doit être vrai (rempli).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<(String, Option<String>)>,
    /// Tri : chemin (`date`, `data.title`) et ordre décroissant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<(String, bool)>,
    /// Nombre maximal de contenus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// Sans le contenu affiché par la page (« autres projets » sur la page d'un projet).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclude_current: bool,
    /// Contenus choisis à la main (identifiants `data.slug`), dans l'ordre affiché ; vide :
    /// tous les contenus.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pick: Vec<String>,
    /// Filtre « contient » : chemin d'un champ liste (`data.expertises`) et valeur cherchée
    /// (expression : `slug`, `page.fileSlug`, `"design"`) ; les projets d'une expertise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<(String, String)>,
    /// Contenus mis en premier, puis les autres dans l'ordre de la liste : champ du contenu
    /// affiché (`suggestions`) ou contenus choisis (`["a", "b"]`) ; avec un nombre maximal, les
    /// plus récents complètent les choisis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,
    /// Contenus mis en premier seulement s'ils remplissent la liste (nombre maximal atteint) ;
    /// sinon la liste ordinaire.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub first_full: bool,
}

/// Contenus mis en premier écrits à la main : `["a", "b"]` -> identifiants.
pub fn first_picks(e: &str) -> Option<Vec<String>> {
    let inner = e.trim().strip_prefix('[')?.strip_suffix(']')?;
    split_top(inner, ',')
        .iter()
        .filter(|x| !x.is_empty())
        .map(|x| crate::canvas::literal(x))
        .collect()
}

/// Liste écrite des contenus mis en premier.
pub fn first_expr(picks: &[String]) -> String {
    let list: Vec<String> = picks.iter().map(|p| crate::canvas::quote(p)).collect();
    format!("[{}]", list.join(", "))
}

fn ident(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Chemin dans un contenu : identifiants séparés par des points.
fn valid_path(p: &str) -> bool {
    !p.is_empty() && p.split('.').all(ident)
}

/// Découpe `s` sur `sep` hors des guillemets et des parenthèses.
fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut esc = false;
    for c in s.chars() {
        if let Some(q) = quote {
            cur.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                cur.push(c);
            }
            '(' | '[' => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            c if c == sep && depth == 0 => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|p| p.trim().to_string()).collect()
}

/// Parenthèses et crochets équilibrés hors des guillemets.
fn balanced(s: &str) -> bool {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut esc = false;
    for c in s.chars() {
        if let Some(q) = quote {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' => depth += 1,
            ')' | ']' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0 && quote.is_none()
}

/// Valeur littérale (texte entre guillemets, nombre, booléen).
fn literal_value(v: &str) -> bool {
    let v = v.trim();
    crate::canvas::literal(v).is_some() || matches!(v, "true" | "false") || v.parse::<f64>().is_ok()
}

impl Query {
    /// Expression de la boucle.
    pub fn to_expr(&self) -> String {
        let base = if ident(&self.collection) {
            format!("collections.{}", self.collection)
        } else {
            format!("collections[{}]", crate::canvas::quote(&self.collection))
        };
        let mut e = base;
        if !self.pick.is_empty() {
            let list: Vec<String> = self.pick.iter().map(|p| crate::canvas::quote(p)).collect();
            e.push_str(&format!(" | by_slugs([{}])", list.join(", ")));
        }
        if let Some((path, value)) = &self.filter {
            match value {
                Some(v) => e.push_str(&format!(
                    " | selectattr({}, \"equalto\", {v})",
                    crate::canvas::quote(path)
                )),
                None => e.push_str(&format!(" | selectattr({})", crate::canvas::quote(path))),
            }
        }
        if let Some((path, value)) = &self.contains {
            e.push_str(&format!(
                " | where_contains({}, {value})",
                crate::canvas::quote(path)
            ));
        }
        if self.exclude_current {
            e.push_str(" | without(page.url)");
        }
        if let Some((path, desc)) = &self.sort {
            if *desc {
                e.push_str(&format!(
                    " | sort(attribute={}, reverse=true)",
                    crate::canvas::quote(path)
                ));
            } else {
                e.push_str(&format!(
                    " | sort(attribute={})",
                    crate::canvas::quote(path)
                ));
            }
        }
        if let Some(f) = &self.first {
            match (self.first_full, self.limit) {
                (true, Some(n)) => e.push_str(&format!(" | pick_first({f}, {n})")),
                _ => e.push_str(&format!(" | pick_first({f})")),
            }
        }
        match self.limit {
            Some(n) => format!("({e})[:{n}]"),
            None => e,
        }
    }

    /// Réglages d'une expression écrite par [`Query::to_expr`] ; `None` pour une autre forme.
    pub fn parse(expr: &str) -> Option<Query> {
        let mut e = expr.trim().to_string();
        let mut limit = None;
        if e.starts_with('(')
            && let Some(i) = e.rfind(")[:")
            && e.ends_with(']')
        {
            let n = e[i + 3..e.len() - 1].trim().parse::<usize>().ok()?;
            let inner = e[1..i].to_string();
            // La parenthèse ouvrante répond à celle qui précède `[:n]`.
            if !balanced(&inner) {
                return None;
            }
            limit = Some(n);
            e = inner;
        }
        let parts = split_top(&e, '|');
        let (base, filters) = parts.split_first()?;
        let collection = if let Some(id) = base.strip_prefix("collections.") {
            ident(id).then(|| id.to_string())?
        } else {
            let inner = base.strip_prefix("collections[")?.strip_suffix(']')?;
            crate::canvas::literal(inner)?
        };
        let mut q = Query {
            collection,
            limit,
            ..Default::default()
        };
        for f in filters {
            let (name, args) = f.split_once('(')?;
            let args = args.strip_suffix(')')?;
            let args = split_top(args, ',');
            match name.trim() {
                "by_slugs"
                    if q.pick.is_empty()
                        && q.filter.is_none()
                        && q.sort.is_none()
                        && !q.exclude_current
                        && args.len() == 1 =>
                {
                    let inner = args[0].trim().strip_prefix('[')?.strip_suffix(']')?;
                    let picks: Option<Vec<String>> = split_top(inner, ',')
                        .iter()
                        .filter(|x| !x.is_empty())
                        .map(|x| crate::canvas::literal(x))
                        .collect();
                    q.pick = picks?;
                    if q.pick.is_empty() {
                        return None;
                    }
                }
                "selectattr"
                    if q.filter.is_none()
                        && q.sort.is_none()
                        && !q.exclude_current
                        && q.first.is_none() =>
                {
                    let path = crate::canvas::literal(args.first()?)?;
                    if !valid_path(&path) {
                        return None;
                    }
                    let value = match args.as_slice() {
                        [_] => None,
                        [_, test, v]
                            if crate::canvas::literal(test).as_deref() == Some("equalto")
                                && literal_value(v) =>
                        {
                            Some(v.trim().to_string())
                        }
                        _ => return None,
                    };
                    q.filter = Some((path, value));
                }
                "where_contains"
                    if q.contains.is_none()
                        && q.first.is_none()
                        && q.sort.is_none()
                        && !q.exclude_current
                        && args.len() == 2 =>
                {
                    let path = crate::canvas::literal(&args[0])?;
                    let v = args[1].trim();
                    if !valid_path(&path) || !(literal_value(v) || valid_path(v)) {
                        return None;
                    }
                    q.contains = Some((path, v.to_string()));
                }
                "without"
                    if !q.exclude_current
                        && q.first.is_none()
                        && q.sort.is_none()
                        && args.len() == 1
                        && args[0] == "page.url" =>
                {
                    q.exclude_current = true;
                }
                "pick_first" if q.first.is_none() && (1..=2).contains(&args.len()) => {
                    let a = args[0].trim();
                    if !(valid_path(a) || first_picks(a).is_some()) {
                        return None;
                    }
                    q.first = Some(a.to_string());
                    if let Some(n) = args.get(1) {
                        // Minimum : le nombre maximal de la liste (relu après).
                        n.trim().parse::<usize>().ok()?;
                        q.first_full = true;
                    }
                }
                "sort" if q.sort.is_none() && q.first.is_none() => {
                    let mut path = None;
                    let mut desc = false;
                    for a in &args {
                        let (k, v) = a.split_once('=')?;
                        match (k.trim(), v.trim()) {
                            ("attribute", v) => path = Some(crate::canvas::literal(v)?),
                            ("reverse", "true") => desc = true,
                            ("reverse", "false") => desc = false,
                            _ => return None,
                        }
                    }
                    let path = path?;
                    if !valid_path(&path) {
                        return None;
                    }
                    q.sort = Some((path, desc));
                }
                _ => return None,
            }
        }
        // `pick_first(f, n)` : n est le nombre maximal de la liste.
        if q.first_full {
            let n = q.limit?;
            if !e.contains(&format!(", {n})")) {
                return None;
            }
        }
        Some(q)
    }
}

/// Liste tirée d'un champ du contenu (liste dans une liste, comme une liste de collection de
/// Webflow sur un champ « référence multiple ») : valeurs du champ (`item.data.tags`), ou contenus
/// d'une collection désignés par leurs adresses (`collections.expertises |
/// by_slugs(item.data.expertises)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldList {
    Values { field: String },
    Refs { collection: String, field: String },
}

impl FieldList {
    pub fn to_expr(&self) -> String {
        match self {
            Self::Values { field } => field.clone(),
            Self::Refs { collection, field } => {
                format!("collections.{collection} | by_slugs({field})")
            }
        }
    }

    /// Forme écrite par [`FieldList::to_expr`] ; `None` pour une autre expression.
    pub fn parse(expr: &str) -> Option<FieldList> {
        let e = expr.trim();
        if let Some((base, f)) = e.split_once('|') {
            let id = base.trim().strip_prefix("collections.")?;
            let field = f
                .trim()
                .strip_prefix("by_slugs(")?
                .strip_suffix(')')?
                .trim();
            return (ident(id) && valid_path(field)).then(|| FieldList::Refs {
                collection: id.to_string(),
                field: field.to_string(),
            });
        }
        (valid_path(e) && !e.starts_with("collections.") && e != "collections").then(|| {
            FieldList::Values {
                field: e.to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour() {
        let qs = [
            Query {
                collection: "projets".into(),
                ..Default::default()
            },
            Query {
                collection: "projets".into(),
                filter: Some(("data.une".into(), None)),
                sort: Some(("date".into(), true)),
                limit: Some(3),
                exclude_current: false,
                pick: Vec::new(),
                contains: None,
                first: None,
                first_full: false,
            },
            Query {
                collection: "actualites-fr".into(),
                filter: Some(("data.categorie".into(), Some("\"design, web\"".into()))),
                sort: Some(("data.title".into(), false)),
                limit: None,
                exclude_current: false,
                pick: Vec::new(),
                contains: None,
                first: None,
                first_full: false,
            },
            Query {
                collection: "projets".into(),
                sort: Some(("date".into(), true)),
                limit: Some(3),
                exclude_current: true,
                ..Default::default()
            },
            // Contenus choisis à la main, dans l'ordre.
            Query {
                collection: "projets".into(),
                pick: vec!["showreel".into(), "voeux-2026".into()],
                limit: Some(1),
                ..Default::default()
            },
            // Projets d'une expertise (modèle de page d'expertise).
            Query {
                collection: "projets".into(),
                contains: Some(("data.expertises".into(), "slug".into())),
                sort: Some(("date".into(), true)),
                limit: Some(3),
                ..Default::default()
            },
            // Suggestions du contenu affiché, complétées par les plus récents.
            Query {
                collection: "projets".into(),
                sort: Some(("date".into(), true)),
                exclude_current: true,
                first: Some("suggestions".into()),
                limit: Some(3),
                ..Default::default()
            },
            Query {
                collection: "projets".into(),
                sort: Some(("date".into(), true)),
                exclude_current: true,
                first: Some("suggestions".into()),
                first_full: true,
                limit: Some(3),
                ..Default::default()
            },
            Query {
                collection: "actus".into(),
                first: Some(first_expr(&["a".into(), "b".into()])),
                limit: Some(3),
                ..Default::default()
            },
        ];
        for q in qs {
            let e = q.to_expr();
            assert_eq!(Query::parse(&e).as_ref(), Some(&q), "{e}");
        }
        assert_eq!(
            Query {
                collection: "projets".into(),
                filter: Some(("data.une".into(), None)),
                sort: Some(("date".into(), true)),
                limit: Some(3),
                exclude_current: false,
                pick: Vec::new(),
                contains: None,
                first: None,
                first_full: false,
            }
            .to_expr(),
            r#"(collections.projets | selectattr("data.une") | sort(attribute="date", reverse=true))[:3]"#
        );
        for f in [
            FieldList::Values {
                field: "item.data.tags".into(),
            },
            FieldList::Refs {
                collection: "expertises".into(),
                field: "expertises".into(),
            },
        ] {
            assert_eq!(FieldList::parse(&f.to_expr()), Some(f.clone()));
        }
        assert_eq!(FieldList::parse("collections.projets"), None);
        assert_eq!(FieldList::parse("collections.a | by_slugs(x | y)"), None);
        for other in [
            "collections.projets | reverse",
            "home.featured",
            "(collections.a)[:x]",
            "collections.projets | sort(attribute=\"date\") | selectattr(\"data.x\")",
        ] {
            assert_eq!(Query::parse(other), None, "{other}");
        }
    }
}
