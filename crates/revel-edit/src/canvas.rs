//! Repères de la page rendue pour l'éditeur (étape 2.2).
//!
//! Dans le HTML du canevas :
//! - chaque élément porte `data-rv="<identifiant du nœud>"` ;
//! - un texte, une expression affichée, un composant utilisé ou une image remplacée par un
//!   composant est entouré de commentaires `<!--rv:N-->` … `<!--/rv:N-->`, où `N` est la position
//!   du repère dans la liste [`Slot`] renvoyée avec la page.
//!
//! Les repères ne changent pas l'affichage : sans eux, la page est identique octet pour octet à la
//! page générée (vérifié par les essais de `revel-render`).

use serde::{Deserialize, Serialize};

use crate::address::{Owner, Parent};

/// Repère d'une portion de page sans élément propre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "slot", rename_all = "snake_case")]
pub enum Slot {
    /// Texte écrit dans une page, un gabarit ou un composant ; modifiable par `SetText`.
    Text { parent: Parent, index: usize },
    /// Valeur d'une expression (`{{ … }}`). `binding` : origine de la valeur quand elle vient
    /// directement d'un fichier de données, d'un contenu ou d'une variable de page (D-19).
    Expr {
        parent: Parent,
        index: usize,
        expr: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        binding: Option<Binding>,
        /// Valeur HTML (`| safe`) : texte riche, pas un simple texte.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        html: bool,
        /// Valeur affichée transformée par des filtres (`| accent`, `| date_fr`…) : la source se
        /// modifie dans un champ, pas directement sur la page.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        transformed: bool,
    },
    /// Composant utilisé (`Use`) ou image remplacée par le composant `image-placeholder`.
    Block { owner: Owner, node: String },
}

/// Origine d'une valeur affichée.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Binding {
    pub source: Source,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<Seg>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Source {
    /// Données globales (dossier de données : `pages.accueil.hero.title` = fichier
    /// `pages/accueil.yml`, clé `hero.title`, selon l'organisation du dossier).
    Data,
    /// Variable d'une page du site (`site.revel.json`).
    Page { page: String },
    /// Variable d'un gabarit.
    Layout { layout: String },
    /// En-tête (front matter) d'un contenu de collection.
    Entry { folder: String, file: String },
    /// Élément de collection (`{ url, data, content, fileSlug, date }`) : seul `data.…` est relié
    /// à l'en-tête du contenu.
    EntryItem { folder: String, file: String },
}

/// Étape d'un chemin dans une valeur.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Seg {
    Index(usize),
    Key(String),
}

impl Binding {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            path: Vec::new(),
        }
    }

    /// Origine d'une valeur atteinte depuis celle-ci par `segs`.
    pub fn extend(&self, segs: &[Seg]) -> Option<Binding> {
        match &self.source {
            Source::EntryItem { folder, file } if self.path.is_empty() => {
                match segs.split_first() {
                    None => Some(self.clone()),
                    Some((Seg::Key(k), rest)) if k == "data" => Some(Binding {
                        source: Source::Entry {
                            folder: folder.clone(),
                            file: file.clone(),
                        },
                        path: rest.to_vec(),
                    }),
                    // `url`, `content`, `fileSlug`, `date` : valeurs calculées.
                    Some(_) => None,
                }
            }
            _ => {
                let mut b = self.clone();
                b.path.extend_from_slice(segs);
                Some(b)
            }
        }
    }
}

/// Expression qui se réduit à un chemin (`home.hero.title`, `a["b"][0]`), éventuellement suivie
/// de `| safe` : renvoie la variable de départ et le chemin. Toute autre expression (filtres,
/// opérations, appels) -> `None`.
pub fn path_expr(expr: &str) -> Option<(String, Vec<Seg>)> {
    let s = expr.trim();
    let (s, _) = match s.rsplit_once('|') {
        Some((head, f)) if f.trim() == "safe" => (head.trim_end(), true),
        Some(_) => return None,
        None => (s, false),
    };
    let b = s.as_bytes();
    let ident = |i: usize| -> Option<usize> {
        if i < b.len() && (b[i].is_ascii_alphabetic() || b[i] == b'_') {
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                j += 1;
            }
            Some(j)
        } else {
            None
        }
    };
    let end = ident(0)?;
    let head = s[..end].to_string();
    if matches!(head.as_str(), "true" | "false" | "none" | "None" | "loop") {
        return None;
    }
    let mut segs = Vec::new();
    let mut i = end;
    while i < b.len() {
        match b[i] {
            b'.' => {
                let j = ident(i + 1)?;
                segs.push(Seg::Key(s[i + 1..j].to_string()));
                i = j;
            }
            b'[' => {
                let close = i + s[i..].find(']')?;
                let inner = s[i + 1..close].trim();
                if let Ok(n) = inner.parse::<usize>() {
                    segs.push(Seg::Index(n));
                } else {
                    let q = inner.chars().next()?;
                    if (q != '"' && q != '\'') || inner.len() < 2 || !inner.ends_with(q) {
                        return None;
                    }
                    let key = &inner[1..inner.len() - 1];
                    if key.contains(q) || key.contains('\\') {
                        return None;
                    }
                    segs.push(Seg::Key(key.to_string()));
                }
                i = close + 1;
            }
            _ => return None,
        }
    }
    Some((head, segs))
}

/// Chemin source d'une expression `chemin | filtre(…) | …` : variable de départ, chemin, et
/// `true` si des filtres autres que `safe` transforment la valeur.
pub fn source_path(expr: &str) -> Option<(String, Vec<Seg>, bool)> {
    // Premier `|` hors chaînes, parenthèses et crochets.
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut cut = None;
    for (i, c) in expr.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                '|' if depth == 0 => {
                    cut = Some(i);
                    break;
                }
                _ => {}
            },
        }
    }
    let Some(cut) = cut else {
        let (h, p) = path_expr(expr)?;
        return Some((h, p, false));
    };
    let (head, segs) = path_expr(&expr[..cut])?;
    let filters = expr[cut + 1..].trim();
    let only_safe = filters.split('|').all(|f| f.trim() == "safe");
    Some((head, segs, !only_safe))
}

/// Texte saisi dans le canevas ramené à l'écriture de la source : la typographie française de la
/// page (espaces insécables, apostrophe typographique) n'est pas recopiée dans la source si celle-ci
/// ne l'utilisait pas (`old` : texte source avant modification).
pub fn untypo(new: &str, old: &str) -> String {
    let mut s = new.to_string();
    if !old.contains('\u{a0}') {
        s = s.replace('\u{a0}', " ");
    }
    if !old.contains('\u{202f}') {
        s = s.replace('\u{202f}', " ");
    }
    if !old.contains('’') {
        s = s.replace('’', "'");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemins() {
        let k = |s: &str| Seg::Key(s.into());
        assert_eq!(
            path_expr(" home.hero.title "),
            Some(("home".into(), vec![k("hero"), k("title")]))
        );
        assert_eq!(
            path_expr("a['b c'][2] | safe"),
            Some(("a".into(), vec![k("b c"), Seg::Index(2)]))
        );
        for e in [
            "a | upper",
            "a ~ b",
            "f(a)",
            "a.b()",
            "a[i]",
            "1",
            "'x'",
            "loop.index",
            "a.",
        ] {
            assert_eq!(path_expr(e), None, "{e}");
        }
    }

    #[test]
    fn chemins_sources() {
        let k = |s: &str| Seg::Key(s.into());
        assert_eq!(
            source_path("a.title | accent | safe"),
            Some(("a".into(), vec![k("title")], true))
        );
        assert_eq!(
            source_path("a.b | safe"),
            Some(("a".into(), vec![k("b")], false))
        );
        assert_eq!(
            source_path("a | f('x|y')"),
            Some(("a".into(), vec![], true))
        );
        assert_eq!(source_path("(a or b) | date_fr"), None);
        assert_eq!(source_path("'' | year"), None);
    }

    #[test]
    fn typographie_retiree() {
        assert_eq!(
            untypo("L’agence\u{a0}: oui", "L'agence : non"),
            "L'agence : oui"
        );
        assert_eq!(
            untypo("L’agence\u{a0}!", "L’agence !"),
            "L’agence !",
            "apostrophe gardée"
        );
        assert_eq!(
            untypo("a\u{a0}b", "a\u{a0}c"),
            "a\u{a0}b",
            "insécable voulue"
        );
    }

    #[test]
    fn element_de_collection() {
        let item = Binding::new(Source::EntryItem {
            folder: "content/projets".into(),
            file: "wattr".into(),
        });
        let t = item
            .extend(&[Seg::Key("data".into()), Seg::Key("title".into())])
            .unwrap();
        assert_eq!(
            t,
            Binding {
                source: Source::Entry {
                    folder: "content/projets".into(),
                    file: "wattr".into()
                },
                path: vec![Seg::Key("title".into())]
            }
        );
        assert_eq!(item.extend(&[Seg::Key("url".into())]), None);
        // JSON compact : indices en nombres, clés en chaînes.
        assert_eq!(
            serde_json::to_string(&t.path).unwrap(),
            r#"["title"]"#.to_string()
        );
    }
}
