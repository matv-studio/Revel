//! Valeurs d'attribut reliées (Inspecteur › Attributs) : texte écrit autour d'une valeur du
//! contenu, d'une propriété ou d'une information du site, éventuellement seulement quand la
//! valeur est remplie ; propriétés de style reliées (`style="--dot:{{ couleur }}"`, comme la
//! couleur d'un champ du CMS liée à un style dans Webflow).
//!
//! Formes écrites :
//! - `avant{{ valeur }}après` ;
//! - `fixe{% if valeur %}avant{{ valeur }}après{% endif %}fin` (« seulement si remplie ») ;
//! - `style` : déclarations `propriété: avant{{ valeur }}après` séparées par `;`, à côté de
//!   déclarations écrites.

use serde::{Deserialize, Serialize};

/// Valeur d'attribut reliée.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bound {
    /// Texte toujours écrit avant (forme « seulement si remplie »).
    pub fixed: String,
    /// Texte écrit juste avant la valeur.
    pub before: String,
    /// Expression de la valeur (`item.data.titre`, `label`, `site.email`, `video | youtube_id`).
    pub value: String,
    /// Texte écrit juste après la valeur.
    pub after: String,
    /// Partie reliée (avant, valeur, après) écrite seulement si la valeur est remplie.
    pub only_if: bool,
    /// Texte toujours écrit après (forme « seulement si remplie »).
    pub tail: String,
}

/// Chemin testé par « seulement si remplie » : la valeur sans son écriture (`x | date(…)` -> `x`).
pub fn head(value: &str) -> &str {
    value.split('|').next().unwrap_or(value).trim()
}

fn plain(s: &str) -> bool {
    !s.contains('{') && !s.contains('}')
}

/// `{{ e }}` au début de `s` : (expression, reste).
fn take_out(s: &str) -> Option<(String, &str)> {
    let r = s.strip_prefix("{{")?;
    let end = r.find("}}")?;
    let e = r[..end].trim().trim_matches('-').trim();
    (!e.is_empty() && !e.contains("{{")).then(|| (e.to_string(), &r[end + 2..]))
}

/// `{% mot args %}` au début de `s` : (instruction, reste).
fn take_stmt(s: &str) -> Option<(String, &str)> {
    let r = s.strip_prefix("{%")?;
    let end = r.find("%}")?;
    Some((
        r[..end].trim().trim_matches('-').trim().to_string(),
        &r[end + 2..],
    ))
}

impl Bound {
    /// Valeur reliée lue dans un attribut ; `None` pour un texte écrit ou une autre forme.
    pub fn parse(v: &str) -> Option<Bound> {
        let i = v.find('{')?;
        let (lead, rest) = v.split_at(i);
        if !plain(lead) {
            return None;
        }
        if let Some((value, after_out)) = take_out(rest) {
            // `avant{{ valeur }}après`.
            return plain(after_out).then(|| Bound {
                before: lead.to_string(),
                value,
                after: after_out.to_string(),
                ..Default::default()
            });
        }
        // `fixe{% if valeur %}avant{{ valeur }}après{% endif %}fin`.
        let (stmt, r) = take_stmt(rest)?;
        let cond = stmt.strip_prefix("if ")?.trim();
        let j = r.find('{')?;
        let (before, r) = r.split_at(j);
        let (value, r) = take_out(r)?;
        let k = r.find('{')?;
        let (after, r) = r.split_at(k);
        let (end, tail) = take_stmt(r)?;
        (plain(before) && plain(after) && plain(tail) && end == "endif" && head(&value) == cond)
            .then(|| Bound {
                fixed: lead.to_string(),
                before: before.to_string(),
                value,
                after: after.to_string(),
                only_if: true,
                tail: tail.to_string(),
            })
    }

    /// Attribut écrit.
    pub fn write(&self) -> String {
        let out = format!(
            "{}{{{{ {} }}}}{}",
            self.before,
            self.value.trim(),
            self.after
        );
        if self.only_if {
            format!(
                "{}{{% if {} %}}{out}{{% endif %}}{}",
                self.fixed,
                head(&self.value),
                self.tail
            )
        } else {
            format!("{}{out}{}", self.fixed, self.tail)
        }
    }

    /// Texte affiché quand la valeur est vide (forme « seulement si remplie »), sinon `None`.
    pub fn when_empty(&self) -> Option<String> {
        self.only_if.then(|| format!("{}{}", self.fixed, self.tail))
    }
}

/// Déclaration d'un attribut `style` : propriété et valeur écrite ou reliée.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleDecl {
    pub prop: String,
    /// Valeur écrite (sans `{{ }}`).
    pub text: Option<String>,
    /// Valeur reliée (avant, valeur, après ; jamais « seulement si remplie »).
    pub bound: Option<Bound>,
}

/// Déclarations d'un attribut `style` ; `None` si une valeur n'est ni écrite ni reliée.
pub fn style_decls(v: &str) -> Option<Vec<StyleDecl>> {
    if v.contains("{%") {
        return None;
    }
    let mut out = Vec::new();
    for part in split_decls(v) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (prop, value) = part.split_once(':')?;
        let prop = prop.trim();
        if prop.is_empty()
            || !prop
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return None;
        }
        let value = value.trim();
        if value.contains("{{") {
            let b = Bound::parse(value).filter(|b| !b.only_if)?;
            out.push(StyleDecl {
                prop: prop.to_string(),
                text: None,
                bound: Some(b),
            });
        } else {
            out.push(StyleDecl {
                prop: prop.to_string(),
                text: Some(value.to_string()),
                bound: None,
            });
        }
    }
    Some(out)
}

/// `;` hors des `{{ }}`, des parenthèses et des guillemets.
fn split_decls(v: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut quote, mut start) = (0i32, None::<char>, 0usize);
    let b: Vec<(usize, char)> = v.char_indices().collect();
    let mut k = 0;
    while k < b.len() {
        let (i, c) = b[k];
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '(' | '{' | '[' => depth += 1,
                ')' | '}' | ']' => depth -= 1,
                ';' if depth == 0 => {
                    parts.push(&v[start..i]);
                    start = i + 1;
                }
                _ => {}
            },
        }
        k += 1;
    }
    parts.push(&v[start..]);
    parts
}

/// Attribut `style` écrit.
pub fn write_style(decls: &[StyleDecl]) -> String {
    decls
        .iter()
        .map(|d| match (&d.bound, &d.text) {
            (Some(b), _) => format!("{}:{}", d.prop, b.write()),
            (None, Some(t)) => format!("{}:{}", d.prop, t),
            (None, None) => format!("{}:", d.prop),
        })
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valeurs_reliees() {
        let b = Bound::parse("Lire la vidéo{% if label %} : {{ label }}{% endif %}").unwrap();
        assert!(b.only_if);
        assert_eq!(b.fixed, "Lire la vidéo");
        assert_eq!(b.before, " : ");
        assert_eq!(b.value, "label");
        assert_eq!(
            b.write(),
            "Lire la vidéo{% if label %} : {{ label }}{% endif %}"
        );
        assert_eq!(b.when_empty().as_deref(), Some("Lire la vidéo"));
        let b = Bound::parse("https://embed/{{ video | youtube_id }}?autoplay=1").unwrap();
        assert!(!b.only_if);
        assert_eq!(b.value, "video | youtube_id");
        assert_eq!(
            b.write(),
            "https://embed/{{ video | youtube_id }}?autoplay=1"
        );
        // Condition sur une autre valeur, deux valeurs : autres formes.
        assert!(Bound::parse("a{% if x %} {{ y }}{% endif %}").is_none());
        assert!(Bound::parse("{{ a }} {{ b }}").is_none());
        assert!(Bound::parse("texte").is_none());
        // Écriture de date : condition sur le champ seul.
        let b = Bound::parse("{% if d %}le {{ d | date(\"%d.%m\") }}{% endif %}").unwrap();
        assert_eq!(head(&b.value), "d");
    }

    #[test]
    fn styles_relies() {
        let d = style_decls("--dot:{{ t.color }}").unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].prop, "--dot");
        assert_eq!(d[0].bound.as_ref().unwrap().value, "t.color");
        let d = style_decls("color: red; --count:{{ collections.equipe|length }};").unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].text.as_deref(), Some("red"));
        assert_eq!(
            write_style(&d),
            "color:red;--count:{{ collections.equipe|length }}"
        );
        let d = style_decls("--dot:{{ ({\"a\":\"#fff\"})[c] }}").unwrap();
        assert_eq!(d[0].bound.as_ref().unwrap().value, "({\"a\":\"#fff\"})[c]");
        assert!(style_decls("--x:{% if a %}1{% endif %}").is_none());
    }
}
