//! Valeurs liées (D-19) : un texte affiché qui vient d'un fichier de données (YAML), de l'en-tête
//! d'un contenu (front matter) ou d'une variable de page ou de gabarit se modifie dans sa source.
//!
//! Dans un fichier, seule la valeur visée est réécrite (écriture ciblée) : le reste du fichier —
//! commentaires, ordre, guillemets, retours à la ligne — est conservé octet pour octet, et le
//! fichier reste lisible par Pages CMS. Le style d'écriture de la valeur est gardé quand c'est
//! possible (sans guillemets, guillemets simples ou doubles) ; sinon, guillemets doubles.

use granit_parser::{Event, Parser, ScalarStyle};
use revel_edit::canvas::{Binding, Seg, Source};
use revel_model::Site;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Échec d'une écriture ciblée (message pour l'utilisateur, en français ; traduit par la clé
/// `error-edit-bound`).
pub type BoundError = String;

#[derive(Debug, Clone, PartialEq)]
enum Frame {
    Map {
        key: Option<String>,
        expect_key: bool,
    },
    Seq {
        index: usize,
    },
}

/// Valeur scalaire trouvée : position (octets), style, valeur.
#[derive(Debug, Clone, PartialEq)]
pub struct Scalar {
    pub start: usize,
    pub end: usize,
    pub style: ScalarStyle,
    pub value: String,
}

/// Cherche la valeur scalaire au chemin `path` dans un document YAML.
pub fn find_scalar(yaml: &str, path: &[Seg]) -> Result<Option<Scalar>, BoundError> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut cur: Vec<Seg> = Vec::new();
    // Étape du chemin d'un nœud valeur dans son parent.
    fn step(stack: &mut [Frame]) -> Option<Seg> {
        match stack.last_mut()? {
            Frame::Map { key, expect_key } => {
                *expect_key = true;
                key.take().map(Seg::Key)
            }
            Frame::Seq { index } => {
                *index += 1;
                Some(Seg::Index(*index - 1))
            }
        }
    }
    for ev in Parser::new_from_str(yaml) {
        let (ev, span) = ev.map_err(|e| format!("YAML illisible : {e}"))?;
        match ev {
            Event::Scalar(v, style, _, _) => {
                if let Some(Frame::Map { key, expect_key }) = stack.last_mut()
                    && *expect_key
                {
                    *key = Some(v.into_owned());
                    *expect_key = false;
                    continue;
                }
                let seg = step(&mut stack);
                let at = cur.len();
                cur.extend(seg);
                if cur == path {
                    let (Some(start), Some(end)) =
                        (span.start.byte_offset(), span.end.byte_offset())
                    else {
                        return Err("position de la valeur inconnue".into());
                    };
                    return Ok(Some(Scalar {
                        start,
                        end,
                        style,
                        value: v.into_owned(),
                    }));
                }
                cur.truncate(at);
            }
            Event::MappingStart(..) | Event::SequenceStart(..) => {
                let is_map = matches!(ev, Event::MappingStart(..));
                // Clé complexe (collection en position de clé) : ignorée.
                if let Some(Frame::Map {
                    expect_key: true, ..
                }) = stack.last()
                {
                    return Err("clé complexe non prise en charge".into());
                }
                let seg = step(&mut stack);
                cur.extend(seg);
                stack.push(if is_map {
                    Frame::Map {
                        key: None,
                        expect_key: true,
                    }
                } else {
                    Frame::Seq { index: 0 }
                });
            }
            Event::MappingEnd | Event::SequenceEnd => {
                stack.pop();
                // Le chemin du parent : retirer l'étape de cette collection (sauf à la racine).
                if !stack.is_empty() {
                    cur.pop();
                }
            }
            Event::Alias(_) => {
                step(&mut stack);
            }
            _ => {}
        }
    }
    Ok(None)
}

/// Valeur qui serait lue comme autre chose qu'un texte sans guillemets (nombre, booléen, vide…).
pub(crate) fn special(v: &str) -> bool {
    let l = v.to_ascii_lowercase();
    matches!(
        l.as_str(),
        "" | "~"
            | "null"
            | "true"
            | "false"
            | "yes"
            | "no"
            | "on"
            | "off"
            | "y"
            | "n"
            | ".inf"
            | "-.inf"
            | ".nan"
    ) || v.parse::<f64>().is_ok()
        || v.starts_with("0x")
        || v.starts_with("0o")
        || (v.len() >= 10 && v.as_bytes()[4] == b'-' && v[..4].chars().all(|c| c.is_ascii_digit()))
}

/// Texte écrivable sans guillemets en conservant son type.
pub(crate) fn plain_ok(new: &str, old: &str) -> bool {
    !new.is_empty()
        && !new.contains(['\n', '\r', '\t'])
        && new.trim() == new
        && !new.starts_with([
            '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%',
            '@', '`',
        ])
        && !new.contains(": ")
        && !new.contains(" #")
        && !new.ends_with(':')
        && special(new) == special(old)
}

pub(crate) fn double_quoted(v: &str) -> String {
    let mut out = String::with_capacity(v.len() + 2);
    out.push('"');
    for c in v.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Écriture YAML de `new` à la place de la valeur `old` écrite dans le style `style`.
pub fn render(new: &str, style: ScalarStyle, old: &str) -> String {
    match style {
        ScalarStyle::Plain if plain_ok(new, old) => new.to_string(),
        ScalarStyle::SingleQuoted if !new.contains(['\n', '\r']) => {
            format!("'{}'", new.replace('\'', "''"))
        }
        _ => double_quoted(new),
    }
}

/// Remplace la valeur au chemin `path` ; renvoie le nouveau texte et l'ancienne valeur.
pub fn set_yaml_value(
    yaml: &str,
    path: &[Seg],
    value: &str,
) -> Result<(String, String), BoundError> {
    let s =
        find_scalar(yaml, path)?.ok_or_else(|| "valeur introuvable dans le fichier".to_string())?;
    if matches!(s.style, ScalarStyle::Literal | ScalarStyle::Folded) {
        // Bloc `|` ou `>` : la plage ne couvre que le contenu ; réécrit en guillemets doubles
        // à partir de l'indicateur de bloc.
        let indicator = yaml[..s.start]
            .rfind(['|', '>'])
            .ok_or_else(|| "bloc de texte illisible".to_string())?;
        let end = yaml[..s.end].trim_end_matches(['\n', '\r', ' ']).len();
        let mut out = String::with_capacity(yaml.len());
        out.push_str(&yaml[..indicator]);
        out.push_str(&double_quoted(value));
        out.push_str(&yaml[end..]);
        return Ok((out, s.value));
    }
    let mut out = String::with_capacity(yaml.len() + value.len());
    out.push_str(&yaml[..s.start]);
    out.push_str(&render(value, s.style, &s.value));
    out.push_str(&yaml[s.end..]);
    Ok((out, s.value))
}

/// En-tête d'un fichier Markdown : (début, fin) du YAML entre les lignes `---`.
pub(crate) fn front_matter_range(text: &str) -> Option<(usize, usize)> {
    let first = text.strip_prefix('\u{feff}').map_or(0, |_| 3);
    let rest = &text[first..];
    let open = if rest.starts_with("---\r\n") {
        5
    } else if rest.starts_with("---\n") {
        4
    } else {
        return None;
    };
    let start = first + open;
    let mut pos = start;
    for line in text[start..].split_inclusive('\n') {
        if line.trim_end_matches(['\n', '\r']) == "---" {
            return Some((start, pos));
        }
        pos += line.len();
    }
    None
}

/// Fichier de données d'un chemin `pages.accueil.hero.title` : `pages/accueil.yml` + `hero.title`.
fn data_file(data_dir: &Path, path: &[Seg]) -> Option<(PathBuf, usize)> {
    let mut rel = PathBuf::new();
    for (i, seg) in path.iter().enumerate() {
        let Seg::Key(k) = seg else { return None };
        if k.contains(['/', '\\']) || k == ".." {
            return None;
        }
        for ext in ["yml", "yaml"] {
            let f = data_dir.join(&rel).join(format!("{k}.{ext}"));
            if f.is_file() {
                return Some((f, i + 1));
            }
        }
        rel.push(k);
        if !data_dir.join(&rel).is_dir() {
            return None;
        }
    }
    None
}

pub(crate) fn write_atomic(path: &Path, text: &str) -> Result<(), BoundError> {
    let tmp = path.with_extension("revel-tmp");
    std::fs::write(&tmp, text)
        .and_then(|_| std::fs::rename(&tmp, path))
        .map_err(|e| format!("{} : {e}", path.display()))
}

/// Valeur JSON d'une variable de page ou de gabarit au chemin donné.
fn json_at<'a>(v: &'a mut Value, path: &[Seg]) -> Option<&'a mut Value> {
    let mut cur = v;
    for seg in path {
        cur = match seg {
            Seg::Key(k) => cur.as_object_mut()?.get_mut(k)?,
            Seg::Index(i) => cur.as_array_mut()?.get_mut(*i)?,
        };
    }
    Some(cur)
}

/// Texte d'une valeur simple.
fn scalar_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

impl crate::Project {
    /// Fichier d'une valeur liée, chemin dans le fichier, en-tête (front matter) ou non.
    pub fn source_file(&self, b: &Binding) -> Result<(PathBuf, Vec<Seg>, bool), BoundError> {
        match &b.source {
            Source::Data | Source::LocaleData { .. } => {
                let dir = self
                    .site
                    .data_dir
                    .as_ref()
                    .ok_or_else(|| "pas de dossier de données".to_string())?;
                let mut base = self.dir.join(dir);
                if let Source::LocaleData { locale } = &b.source {
                    base = base.join(self.locale_code(locale)?);
                }
                let (file, used) = data_file(&base, &b.path)
                    .ok_or_else(|| "fichier de données introuvable (YAML seulement)".to_string())?;
                Ok((file, b.path[used..].to_vec(), false))
            }
            Source::Entry { folder, file } => {
                if [folder.as_str(), file.as_str()]
                    .iter()
                    .any(|p| p.split(['/', '\\']).any(|s| s == ".."))
                {
                    return Err("chemin refusé".into());
                }
                let base = self.dir.join(folder);
                let f = ["md", "markdown"]
                    .iter()
                    .map(|e| base.join(format!("{file}.{e}")))
                    .find(|p| p.is_file())
                    .ok_or_else(|| format!("contenu introuvable : {folder}/{file}"))?;
                Ok((f, b.path.clone(), true))
            }
            _ => Err("pas un fichier".into()),
        }
    }

    /// Écrit une valeur dans un fichier de données (`b` : source `data`, chemin jusqu'à la clé ;
    /// `None` : clé retirée). Renvoie le fichier (relatif au projet) et son texte d'avant.
    pub fn set_data_value(
        &self,
        b: &Binding,
        value: Option<&serde_json::Value>,
    ) -> Result<(String, String), BoundError> {
        if !matches!(b.source, Source::Data | Source::LocaleData { .. }) {
            return Err("pas un fichier de données".into());
        }
        let (file, inner, _) = self.source_file(b)?;
        let keys: Vec<String> = inner
            .iter()
            .map(|s| match s {
                Seg::Key(k) => Ok(k.clone()),
                Seg::Index(_) => Err("chemin dans une liste non pris en charge".to_string()),
            })
            .collect::<Result<_, _>>()?;
        let old = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let new = crate::yamlpath::set_path(&old, &keys, value)?;
        if new != old {
            write_atomic(&file, &new)?;
        }
        let rel = file
            .strip_prefix(&self.dir)
            .map_err(|_| "fichier hors du projet".to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        Ok((rel, old))
    }

    /// Code d'une autre langue du site (refusé s'il n'en est pas une).
    fn locale_code<'c>(&self, code: &'c str) -> Result<&'c str, BoundError> {
        if self.site.locale(code).is_some() && revel_model::valid_locale_code(code) {
            Ok(code)
        } else {
            Err(format!("langue inconnue : {code}"))
        }
    }

    /// Dossier de la langue principale d'un dossier de contenus d'une autre langue
    /// (`src/content/en/projets` -> `src/content/projets`).
    pub fn primary_folder(&self, folder: &str) -> Option<String> {
        self.site.collections.iter().find_map(|c| {
            (!c.folder.is_empty()
                && self
                    .site
                    .locales
                    .iter()
                    .any(|l| revel_model::locale_folder(&c.folder, &l.code) == folder))
            .then(|| c.folder.clone())
        })
    }

    /// Contenu d'une autre langue créé, avant d'y écrire, en copiant celui de la langue
    /// principale. Renvoie la commande qui le retire (annuler), `None` s'il existait déjà ou si le
    /// dossier n'est pas celui d'une autre langue.
    pub fn ensure_locale_entry(
        &self,
        folder: &str,
        file: &str,
    ) -> Result<Option<revel_edit::Command>, BoundError> {
        let Some(primary) = self.primary_folder(folder) else {
            return Ok(None);
        };
        if self.entry_text(folder, file)?.is_some() {
            return Ok(None);
        }
        let text = self
            .entry_text(&primary, file)?
            .ok_or_else(|| format!("contenu introuvable : {primary}/{file}"))?;
        self.write_entry(folder, file, Some(&text))?;
        let undo = revel_edit::Command::WriteEntry {
            folder: folder.to_string(),
            file: file.to_string(),
            text: None,
        };
        let code = self
            .site
            .locales
            .iter()
            .find(|l| revel_model::locale_folder(&primary, &l.code) == folder)
            .map(|l| l.code.clone())
            .unwrap_or_default();
        Ok(Some(self.with_pages_cms(undo, &primary, folder, &code)))
    }

    /// Premier fichier d'une autre langue créé : entrée de Pages CMS de la langue (copie de celle
    /// de la langue principale, `projets-en`, « Projets (EN) ») ajoutée à `.pages.yml` s'il décrit
    /// le dossier ou le fichier de la langue principale et pas encore celui de la langue (D-23 :
    /// les langues restent modifiables dans Pages CMS). Renvoie la commande d'annulation complétée.
    fn with_pages_cms(
        &self,
        undo: revel_edit::Command,
        primary: &str,
        local: &str,
        code: &str,
    ) -> revel_edit::Command {
        let added = (|| -> Option<String> {
            let text = crate::pagesyml::read(&self.dir).ok()??;
            let doc = crate::parse_yaml(&text).ok()?;
            let list = doc.get("content")?.as_array()?;
            let norm = |p: &str| p.trim_matches('/').to_string();
            let path_of = |x: &Value| x.get("path").and_then(Value::as_str).map(norm);
            if list.iter().any(|x| path_of(x) == Some(norm(local))) {
                return None;
            }
            let src = list.iter().find(|x| path_of(x) == Some(norm(primary)))?;
            let name = src.get("name")?.as_str()?;
            let taken = |n: &str| {
                list.iter()
                    .any(|x| x.get("name").and_then(Value::as_str) == Some(n))
            };
            let mut new_name = format!("{name}-{code}");
            let mut n = 2;
            while taken(&new_name) {
                new_name = format!("{name}-{code}-{n}");
                n += 1;
            }
            let label = src
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(name)
                .to_string();
            let mut def = src.clone();
            let o = def.as_object_mut()?;
            o.insert("name".into(), Value::String(new_name));
            o.insert(
                "label".into(),
                Value::String(format!("{label} ({})", code.to_uppercase())),
            );
            o.insert("path".into(), Value::String(local.to_string()));
            let new = crate::pagesyml::add_collection(&text, &def).ok()?;
            crate::pagesyml::write(&self.dir, Some(&new)).ok()?;
            Some(text)
        })();
        match added {
            Some(old) => revel_edit::Command::Batch {
                commands: vec![
                    undo,
                    revel_edit::Command::WriteCmsConfig { text: Some(old) },
                ],
            },
            None => undo,
        }
    }

    /// Fichier d'une autre langue créé avant d'y écrire une valeur liée, copie de celui de la
    /// langue principale (fichier de données, contenu). Renvoie la commande qui le retire.
    pub fn ensure_locale_file(
        &self,
        b: &Binding,
    ) -> Result<Option<revel_edit::Command>, BoundError> {
        match &b.source {
            Source::LocaleData { locale } => {
                let code = self.locale_code(locale)?;
                let dir = self
                    .site
                    .data_dir
                    .as_ref()
                    .ok_or_else(|| "pas de dossier de données".to_string())?;
                let base = self.dir.join(dir);
                if data_file(&base.join(code), &b.path).is_some() {
                    return Ok(None);
                }
                let (primary, _) = data_file(&base, &b.path)
                    .ok_or_else(|| "fichier de données introuvable (YAML seulement)".to_string())?;
                let rel = primary
                    .strip_prefix(&base)
                    .map_err(|_| "fichier hors du dossier de données".to_string())?;
                let target = base.join(code).join(rel);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("{} : {e}", parent.display()))?;
                }
                let text = std::fs::read_to_string(&primary).map_err(|e| e.to_string())?;
                write_atomic(&target, &text)?;
                let file = target
                    .strip_prefix(&self.dir)
                    .map_err(|_| "fichier hors du projet".to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let primary_rel = primary
                    .strip_prefix(&self.dir)
                    .map_err(|_| "fichier hors du projet".to_string())?
                    .to_string_lossy()
                    .replace('\\', "/");
                let undo = revel_edit::Command::WriteDataFile {
                    file: file.clone(),
                    text: None,
                };
                Ok(Some(self.with_pages_cms(undo, &primary_rel, &file, code)))
            }
            Source::Entry { folder, file } => self.ensure_locale_entry(folder, file),
            _ => Ok(None),
        }
    }

    /// Fichier de données (relatif au projet, dans le dossier de données) remplacé par `text`
    /// (`None` : retiré, avec les dossiers restés vides) ; renvoie son texte d'avant (`None` : il
    /// n'existait pas).
    pub fn write_data_file(
        &self,
        file: &str,
        text: Option<&str>,
    ) -> Result<Option<String>, BoundError> {
        let dir = self
            .site
            .data_dir
            .as_ref()
            .ok_or_else(|| "pas de dossier de données".to_string())?;
        if file.split(['/', '\\']).any(|s| s == "..")
            || !file
                .replace('\\', "/")
                .starts_with(&format!("{}/", dir.trim_end_matches('/')))
        {
            return Err(format!("fichier refusé : {file}"));
        }
        let path = self.dir.join(file);
        let old = std::fs::read_to_string(&path).ok();
        match text {
            Some(t) => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("{} : {e}", parent.display()))?;
                }
                write_atomic(&path, t)?;
            }
            None => {
                if path.is_file() {
                    std::fs::remove_file(&path).map_err(|e| format!("{} : {e}", path.display()))?;
                }
                // Dossiers restés vides (créés pour ce fichier), jusqu'au dossier de données.
                let root = self.dir.join(dir);
                let mut d = path.parent().map(Path::to_path_buf);
                while let Some(x) = d {
                    if x == root
                        || !x.starts_with(&root)
                        || std::fs::read_dir(&x).map_or(true, |mut r| r.next().is_some())
                    {
                        break;
                    }
                    let _ = std::fs::remove_dir(&x);
                    d = x.parent().map(Path::to_path_buf);
                }
            }
        }
        Ok(old)
    }

    /// Valeur actuelle (texte) d'une valeur liée.
    pub fn bound_value(&self, b: &Binding) -> Result<String, BoundError> {
        match &b.source {
            Source::Page { .. } | Source::Layout { .. } => {
                let mut site = self.site.clone();
                let v = props_value(&mut site, b)?;
                scalar_text(v).ok_or_else(|| "pas un texte".into())
            }
            // Information du site écrite dans le document (`site.revel.json`).
            Source::Data if document_root(&self.site, &b.path) => {
                let v = document_value(&self.site, &b.path).ok_or("donnée introuvable")?;
                scalar_text(v).ok_or_else(|| "pas un texte".into())
            }
            Source::EntryItem { .. } => Err("valeur calculée".into()),
            // Écrite dans l'utilisation du composant (commande `SetUseArg`), pas dans un fichier.
            Source::Prop { .. } => Err("propriété de composant".into()),
            Source::LocaleData { .. } if self.source_file(b).is_err() => {
                // Pas encore traduit : valeur de la langue principale.
                self.bound_value(&Binding {
                    source: Source::Data,
                    path: b.path.clone(),
                })
            }
            Source::Entry { folder, file }
                if self.primary_folder(folder).is_some()
                    && self.entry_text(folder, file).ok().flatten().is_none() =>
            {
                let primary = self.primary_folder(folder).unwrap_or_default();
                self.bound_value(&Binding {
                    source: Source::Entry {
                        folder: primary,
                        file: file.clone(),
                    },
                    path: b.path.clone(),
                })
            }
            _ => {
                let (file, inner, front) = self.source_file(b)?;
                let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
                let yaml = if front {
                    let (a, z) = front_matter_range(&text).ok_or("en-tête absent")?;
                    &text[a..z]
                } else {
                    &text[..]
                };
                Ok(find_scalar(yaml, &inner)?
                    .ok_or("valeur introuvable dans le fichier")?
                    .value)
            }
        }
    }

    /// Écrit une valeur liée ; renvoie l'ancienne valeur. Les variables de page et de gabarit
    /// sont modifiées dans `site` (document en cours d'édition), les autres dans leur fichier.
    pub fn set_bound(
        &self,
        site: &mut Site,
        b: &Binding,
        value: &str,
    ) -> Result<String, BoundError> {
        match &b.source {
            // Information du site écrite dans le document (`site.revel.json`).
            Source::Data if document_root(site, &b.path) => {
                let v = document_value_mut(site, &b.path).ok_or("donnée introuvable")?;
                let old = scalar_text(v).ok_or("pas un texte")?;
                *v = match v {
                    Value::Number(_) => serde_json::from_str::<serde_json::Number>(value.trim())
                        .map(Value::Number)
                        .unwrap_or_else(|_| Value::String(value.to_string())),
                    _ => Value::String(value.to_string()),
                };
                Ok(old)
            }
            Source::Page { .. } | Source::Layout { .. } => {
                let v = props_value(site, b)?;
                let old = scalar_text(v).ok_or("pas un texte")?;
                *v = match v {
                    // Un nombre reste un nombre s'il en est encore un.
                    Value::Number(_) => serde_json::from_str::<serde_json::Number>(value.trim())
                        .map(Value::Number)
                        .unwrap_or_else(|_| Value::String(value.to_string())),
                    _ => Value::String(value.to_string()),
                };
                Ok(old)
            }
            Source::EntryItem { .. } => Err("valeur calculée".into()),
            // Écrite dans l'utilisation du composant (commande `SetUseArg`), pas dans un fichier.
            Source::Prop { .. } => Err("propriété de composant".into()),
            _ => {
                let (file, inner, front) = self.source_file(b)?;
                let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
                let (new_text, old) = if front {
                    let (a, z) = front_matter_range(&text).ok_or("en-tête absent")?;
                    let (yaml, old) = set_yaml_value(&text[a..z], &inner, value)?;
                    (format!("{}{yaml}{}", &text[..a], &text[z..]), old)
                } else {
                    set_yaml_value(&text, &inner, value)?
                };
                if new_text != text {
                    // Le résultat doit rester lisible et donner la nouvelle valeur.
                    let check = if front {
                        let (a, z) = front_matter_range(&new_text).ok_or("en-tête perdu")?;
                        find_scalar(&new_text[a..z], &inner)?
                    } else {
                        find_scalar(&new_text, &inner)?
                    };
                    if check.map(|s| s.value) != Some(value.to_string()) {
                        return Err("écriture refusée : relecture différente".into());
                    }
                    write_atomic(&file, &new_text)?;
                }
                Ok(old)
            }
        }
    }
}

fn props_value<'a>(site: &'a mut Site, b: &Binding) -> Result<&'a mut Value, BoundError> {
    let (Some(Seg::Key(key)), rest) = (b.path.first(), b.path.get(1..).unwrap_or_default()) else {
        return Err("chemin vide".into());
    };
    let props = match &b.source {
        Source::Page { page } => {
            &mut site
                .pages
                .iter_mut()
                .find(|p| &p.id == page)
                .ok_or("page introuvable")?
                .props
        }
        Source::Layout { layout } => {
            &mut site
                .layouts
                .iter_mut()
                .find(|l| &l.id == layout)
                .ok_or("gabarit introuvable")?
                .props
        }
        _ => return Err("pas une variable".into()),
    };
    let v = props.get_mut(key).ok_or("variable introuvable")?;
    json_at(v, rest).ok_or_else(|| "valeur introuvable".into())
}

/// La donnée globale vient-elle du document (`site.data`) plutôt que d'un fichier de données ?
fn document_root(site: &Site, path: &[Seg]) -> bool {
    matches!(path.first(), Some(Seg::Key(k)) if site.data.contains_key(k))
}

fn document_value<'a>(site: &'a Site, path: &[Seg]) -> Option<&'a Value> {
    let (first, rest) = path.split_first()?;
    let Seg::Key(k) = first else { return None };
    let mut cur = site.data.get(k)?;
    for seg in rest {
        cur = match seg {
            Seg::Key(k) => cur.get(k)?,
            Seg::Index(i) => cur.get(*i)?,
        };
    }
    Some(cur)
}

fn document_value_mut<'a>(site: &'a mut Site, path: &[Seg]) -> Option<&'a mut Value> {
    let (first, rest) = path.split_first()?;
    let Seg::Key(k) = first else { return None };
    let mut cur = site.data.get_mut(k)?;
    for seg in rest {
        cur = match seg {
            Seg::Key(k) => cur.get_mut(k)?,
            Seg::Index(i) => cur.get_mut(*i)?,
        };
    }
    Some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(s: &str) -> Seg {
        Seg::Key(s.into())
    }

    const YAML: &str = "# Accueil\nhero:\n  title: Faire rayonner   # commentaire\n  intro: 'L''agence'\n  quote: \"Dire \\\"oui\\\"\"\n  year: 2026\n  items:\n    - label: Un\n    - label: Deux\n  long: |\n    Ligne une\n    ligne deux\nfin: true\n";

    #[test]
    fn valeurs_trouvees() {
        let f = |p: &[Seg]| find_scalar(YAML, p).unwrap().map(|s| s.value);
        assert_eq!(
            f(&[k("hero"), k("title")]).as_deref(),
            Some("Faire rayonner")
        );
        assert_eq!(f(&[k("hero"), k("intro")]).as_deref(), Some("L'agence"));
        assert_eq!(f(&[k("hero"), k("quote")]).as_deref(), Some("Dire \"oui\""));
        assert_eq!(
            f(&[k("hero"), k("items"), Seg::Index(1), k("label")]).as_deref(),
            Some("Deux")
        );
        assert_eq!(f(&[k("fin")]).as_deref(), Some("true"));
        assert_eq!(f(&[k("absent")]), None);
    }

    #[test]
    fn ecriture_ciblee() {
        let set = |p: &[Seg], v: &str| set_yaml_value(YAML, p, v).unwrap();
        // Texte simple : seul le texte change (commentaire et espaces gardés).
        let (t, old) = set(&[k("hero"), k("title")], "Faire briller");
        assert_eq!(old, "Faire rayonner");
        assert_eq!(
            t,
            YAML.replace("title: Faire rayonner", "title: Faire briller")
        );
        // Caractères spéciaux : guillemets doubles.
        let (t, _) = set(&[k("hero"), k("title")], "Oui : non # pas un commentaire");
        assert!(t.contains("title: \"Oui : non # pas un commentaire\"   # commentaire"));
        // Style gardé : guillemets simples, doubles.
        let (t, _) = set(&[k("hero"), k("intro")], "L'atelier");
        assert!(t.contains("intro: 'L''atelier'"));
        let (t, _) = set(&[k("hero"), k("quote")], "Dire \\ \"non\"");
        assert!(t.contains(r#"quote: "Dire \\ \"non\"""#), "{t}");
        // Nombre : reste un nombre ; un texte qui ressemble à un nombre est mis entre guillemets.
        let (t, _) = set(&[k("hero"), k("year")], "2027");
        assert!(t.contains("year: 2027\n"));
        let (t, _) = set(&[k("hero"), k("title")], "2027");
        assert!(t.contains("title: \"2027\""));
        // Élément de liste.
        let (t, _) = set(
            &[k("hero"), k("items"), Seg::Index(0), k("label")],
            "Premier",
        );
        assert!(t.contains("- label: Premier\n    - label: Deux"));
        // Bloc : réécrit en guillemets doubles, la suite du fichier intacte.
        let (t, old) = set(&[k("hero"), k("long")], "Une seule ligne");
        assert_eq!(old, "Ligne une\nligne deux\n");
        assert!(
            t.contains("  long: \"Une seule ligne\"\nfin: true\n"),
            "{t}"
        );
        // Chaque résultat se relit avec la nouvelle valeur.
        for (p, v) in [
            (vec![k("hero"), k("title")], "a: b"),
            (vec![k("hero"), k("intro")], "x'y"),
            (vec![k("hero"), k("long")], "z"),
        ] {
            let (t, _) = set(&p, v);
            assert_eq!(find_scalar(&t, &p).unwrap().unwrap().value, v);
        }
        assert!(set_yaml_value(YAML, &[k("absent")], "x").is_err());
    }

    #[test]
    fn en_tete_de_contenu() {
        let md = "---\ntitle: Wattr\ndate: 2026-01-02\n---\n\nCorps *md*\n";
        let (a, z) = front_matter_range(md).unwrap();
        assert_eq!(&md[a..z], "title: Wattr\ndate: 2026-01-02\n");
        assert_eq!(front_matter_range("Sans en-tête"), None);
    }
}
