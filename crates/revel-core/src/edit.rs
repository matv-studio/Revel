//! Sessions d'édition : un site ouvert dans l'éditeur, son historique, son enregistrement.
//!
//! Chaque modification passe par une commande (`revel-edit`), est appliquée au document en mémoire,
//! puis enregistrée aussitôt dans `site.revel.json` (écriture atomique). L'historique vit le temps
//! de la session.

use crate::{Core, CoreError, Result};
use revel_edit::{Command, EditState, History, Opened};
use revel_project::Project;
use revel_project::pagesyml::{self, FieldOp};
use std::path::Path;
use std::time::SystemTime;

pub(crate) struct Session {
    project: Project,
    history: History,
    revision: u64,
    /// Date de modification de `site.revel.json` après le dernier enregistrement de Revel.
    saved_mtime: Option<SystemTime>,
}

fn mtime(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir.join(revel_project::SITE_FILE))
        .and_then(|m| m.modified())
        .ok()
}

impl Session {
    fn state(&self, applied: Option<Command>) -> EditState {
        EditState {
            revision: self.revision,
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            applied,
        }
    }

    fn save(&mut self) -> Result<()> {
        self.project.save()?;
        self.saved_mtime = mtime(&self.project.dir);
        Ok(())
    }
}

/// Exécutant des commandes d'une session : `SetBound` écrit la source de la valeur (fichier de
/// données, en-tête d'un contenu) ou la variable de page ; le reste passe par `revel_edit::apply`.
/// Expressions écrites par une commande (affectation, condition, boucle) : syntaxe vérifiée avant
/// l'application, pour que la page reste affichable.
fn check_exprs(c: &Command) -> std::result::Result<(), revel_edit::EditError> {
    let check = |e: &str| {
        revel_render::check_expression(e)
            .map_err(|m| revel_edit::EditError::InvalidExpr(format!("{e} : {m}")))
    };
    match c {
        Command::Replace {
            node: revel_model::Node::Set { expr, .. } | revel_model::Node::Out { expr },
            ..
        } => check(expr),
        Command::SetLogic { cond, expr, .. } => {
            cond.iter().chain(expr.iter()).try_for_each(|e| check(e))
        }
        Command::SetUseArg { value: Some(v), .. } => check(v),
        Command::SetFilter {
            filter: Some(f), ..
        } => check(&f.expr),
        Command::Batch { commands } => commands.iter().try_for_each(check_exprs),
        _ => Ok(()),
    }
}

fn exec(
    project: &Project,
    site: &mut revel_model::Site,
    c: &Command,
) -> std::result::Result<Command, revel_edit::EditError> {
    check_exprs(c)?;
    match c {
        // Texte d'une propriété de composant : écrit dans l'utilisation.
        Command::SetBound {
            binding:
                revel_edit::canvas::Binding {
                    source: revel_edit::canvas::Source::Prop { owner, node, name },
                    path,
                },
            value,
        } if path.is_empty() => revel_edit::apply(
            site,
            &Command::SetUseArg {
                at: revel_edit::Target {
                    owner: owner.clone(),
                    node: node.clone(),
                },
                name: name.clone(),
                value: Some(revel_edit::canvas::quote(value)),
            },
        ),
        Command::SetBound { binding, value } => {
            // Autre langue : fichier de la langue créé au besoin (copie) ; annuler le retire.
            let created = project
                .ensure_locale_file(binding)
                .map_err(revel_edit::EditError::Bound)?;
            match project.set_bound(site, binding, value) {
                Ok(old) => Ok(reviewed(
                    site,
                    binding,
                    created.unwrap_or(Command::SetBound {
                        binding: binding.clone(),
                        value: old,
                    }),
                )),
                Err(e) => {
                    if let Some(c) = created {
                        let _ = exec(project, site, &c);
                    }
                    Err(revel_edit::EditError::Bound(e))
                }
            }
        }
        Command::SetEntryField {
            folder,
            file,
            key,
            value,
        } => {
            let created = project
                .ensure_locale_entry(folder, file)
                .map_err(revel_edit::EditError::Bound)?;
            let old = project
                .entry_text(folder, file)
                .map_err(revel_edit::EditError::Bound)?
                .ok_or_else(|| revel_edit::EditError::Bound(format!("{folder}/{file}")))?;
            let new = revel_project::entries::set_field(&old, key, value.as_ref())
                .map_err(revel_edit::EditError::Bound)?;
            let inv = write_entry(project, folder, file, Some(&new), Some(old))?;
            Ok(created.unwrap_or(inv))
        }
        Command::SetEntryBody { folder, file, body } => {
            let created = project
                .ensure_locale_entry(folder, file)
                .map_err(revel_edit::EditError::Bound)?;
            let old = project
                .entry_text(folder, file)
                .map_err(revel_edit::EditError::Bound)?
                .ok_or_else(|| revel_edit::EditError::Bound(format!("{folder}/{file}")))?;
            let new = revel_project::entries::set_body(&old, body);
            let inv = write_entry(project, folder, file, Some(&new), Some(old))?;
            Ok(created.unwrap_or(inv))
        }
        Command::CreateEntry {
            folder,
            file,
            data,
            body,
        } => {
            if project
                .entry_text(folder, file)
                .map_err(revel_edit::EditError::Bound)?
                .is_some()
            {
                return Err(revel_edit::EditError::Bound(format!(
                    "le contenu existe déjà : {folder}/{file}"
                )));
            }
            let text = revel_project::entries::entry_text(data, body);
            write_entry(project, folder, file, Some(&text), None)
        }
        Command::WriteEntry { folder, file, text } => {
            let old = project
                .entry_text(folder, file)
                .map_err(revel_edit::EditError::Bound)?;
            write_entry(project, folder, file, text.as_deref(), old)
        }
        Command::SetCmsField {
            folder,
            field,
            patch,
        } => schema_edit(project, folder, |text, name| {
            pagesyml::edit_fields(
                text,
                name,
                &FieldOp::Set {
                    field: field.clone(),
                    patch: object(patch)?,
                },
            )
        }),
        Command::AddCmsField { folder, def, index } => {
            let def = complete_field(project, folder, def)?;
            schema_edit(project, folder, |text, name| {
                pagesyml::edit_fields(
                    text,
                    name,
                    &FieldOp::Add {
                        def: def.clone(),
                        index: *index,
                    },
                )
            })
        }
        Command::RemoveCmsField { folder, field } => schema_edit(project, folder, |text, name| {
            pagesyml::edit_fields(
                text,
                name,
                &FieldOp::Remove {
                    field: field.clone(),
                },
            )
        }),
        Command::MoveCmsField {
            folder,
            field,
            index,
        } => schema_edit(project, folder, |text, name| {
            pagesyml::edit_fields(
                text,
                name,
                &FieldOp::Move {
                    field: field.clone(),
                    index: *index,
                },
            )
        }),
        Command::SetCmsCollection { folder, patch } => {
            schema_edit(project, folder, |text, name| {
                pagesyml::set_collection(text, name, &object(patch)?)
            })
        }
        Command::CreateCmsCollection {
            id,
            label,
            folder,
            fields,
        } => create_collection(project, site, id, label, folder, fields),
        Command::RemoveCmsCollection { id, config } => {
            use revel_edit::EditError::Bound;
            let i = site
                .collections
                .iter()
                .position(|c| c.id == *id)
                .ok_or_else(|| Bound(format!("collection inconnue : {id}")))?;
            let c = site.collections.remove(i);
            // Libellé et champs relus pour la recréer à l'identique (rétablir).
            let current = pagesyml::read(&project.dir)
                .map_err(Bound)?
                .unwrap_or_default();
            let doc = revel_project::parse_yaml(&current).unwrap_or_default();
            let path = crate::pagescms::norm_path(&c.folder);
            let item = doc
                .get("content")
                .and_then(serde_json::Value::as_array)
                .and_then(|l| {
                    l.iter().find(|x| {
                        x.get("path")
                            .and_then(|p| p.as_str())
                            .is_some_and(|p| crate::pagescms::norm_path(p) == path)
                    })
                })
                .cloned()
                .unwrap_or_default();
            pagesyml::write(&project.dir, config.as_deref()).map_err(Bound)?;
            Ok(Command::CreateCmsCollection {
                id: c.id,
                label: item
                    .get("label")
                    .and_then(|l| l.as_str())
                    .unwrap_or_default()
                    .to_string(),
                folder: c.folder,
                fields: item.get("fields").cloned().unwrap_or(serde_json::json!([])),
            })
        }
        Command::SetDataValue { binding, value } => {
            let created = project
                .ensure_locale_file(binding)
                .map_err(revel_edit::EditError::Bound)?;
            match project.set_data_value(binding, value.as_ref()) {
                Ok((file, old)) => Ok(reviewed(
                    site,
                    binding,
                    created.unwrap_or(Command::WriteDataFile {
                        file,
                        text: Some(old),
                    }),
                )),
                Err(e) => {
                    if let Some(c) = created {
                        let _ = exec(project, site, &c);
                    }
                    Err(revel_edit::EditError::Bound(e))
                }
            }
        }
        Command::WriteDataFile { file, text } => {
            let old = project
                .write_data_file(file, text.as_deref())
                .map_err(revel_edit::EditError::Bound)?;
            Ok(Command::WriteDataFile {
                file: file.clone(),
                text: old,
            })
        }
        Command::MoveFiles { moves } => {
            use revel_edit::EditError::Bound;
            let inside =
                |rel: &str| -> std::result::Result<std::path::PathBuf, revel_edit::EditError> {
                    let p = Path::new(rel);
                    if rel.is_empty()
                        || p.is_absolute()
                        || p.components()
                            .any(|c| !matches!(c, std::path::Component::Normal(_)))
                    {
                        return Err(Bound(format!("chemin refusé : {rel}")));
                    }
                    Ok(project.dir.join(p))
                };
            let mut done: Vec<(String, String)> = Vec::new();
            for (from, to) in moves {
                let r = (|| {
                    let (a, b) = (inside(from)?, inside(to)?);
                    if b.exists() {
                        return Err(Bound(format!("fichier déjà présent : {to}")));
                    }
                    if let Some(d) = b.parent() {
                        std::fs::create_dir_all(d).map_err(|e| Bound(e.to_string()))?;
                    }
                    std::fs::rename(&a, &b).map_err(|e| Bound(format!("{from} : {e}")))
                })();
                if let Err(e) = r {
                    // Déplacements déjà faits remis en place.
                    for (f, t) in done.iter().rev() {
                        let _ = std::fs::rename(project.dir.join(t), project.dir.join(f));
                    }
                    return Err(e);
                }
                done.push((from.clone(), to.clone()));
            }
            // Dossiers vidés retirés (dossier d'une police), jamais un dossier copié tel quel ni
            // le dossier du site.
            let roots: Vec<std::path::PathBuf> = project
                .site
                .passthrough
                .iter()
                .map(|p| project.dir.join(&p.from))
                .collect();
            for (from, _) in &done {
                let mut d = project.dir.join(from);
                while let Some(parent) = d.parent() {
                    if parent == project.dir
                        || roots.iter().any(|r| r == parent)
                        || std::fs::remove_dir(parent).is_err()
                    {
                        break;
                    }
                    d = parent.to_path_buf();
                }
            }
            Ok(Command::MoveFiles {
                moves: done.into_iter().rev().map(|(f, t)| (t, f)).collect(),
            })
        }
        Command::WriteCmsConfig { text } => {
            let old = pagesyml::read(&project.dir).map_err(revel_edit::EditError::Bound)?;
            pagesyml::write(&project.dir, text.as_deref()).map_err(revel_edit::EditError::Bound)?;
            Ok(Command::WriteCmsConfig { text: old })
        }
        // Suite qui écrit des fichiers du projet : chaque commande passe par le cœur ; échec :
        // ce qui a été appliqué est annulé (la suite reste atomique).
        Command::Batch { commands } if commands.iter().any(external) => {
            let mut inverses: Vec<Command> = Vec::with_capacity(commands.len());
            for c in commands {
                match exec(project, site, c) {
                    Ok(inv) => inverses.push(inv),
                    Err(e) => {
                        for inv in inverses.iter().rev() {
                            let _ = exec(project, site, inv);
                        }
                        return Err(e);
                    }
                }
            }
            inverses.reverse();
            Ok(Command::Batch { commands: inverses })
        }
        other => revel_edit::apply(site, other),
    }
}

/// Valeur d'un fichier de données d'une langue écrite à la main : marque « traduction
/// automatique à relire » retirée (D-36), rétablie avec l'inverse.
fn reviewed(
    site: &mut revel_model::Site,
    binding: &revel_edit::canvas::Binding,
    inverse: Command,
) -> Command {
    let revel_edit::canvas::Source::LocaleData { locale } = &binding.source else {
        return inverse;
    };
    let key = revel_edit::locale::data_key(&binding.path);
    let removed = site
        .locales
        .iter_mut()
        .find(|l| l.code == *locale)
        .is_some_and(|l| l.machine.remove(&key));
    if !removed {
        return inverse;
    }
    Command::Batch {
        commands: vec![
            inverse,
            Command::MarkMachine {
                locale: locale.clone(),
                keys: vec![key],
                on: true,
            },
        ],
    }
}

/// Autre langue d'une valeur liée (fichier de données ou contenu d'une langue), `None` : langue
/// principale.
fn binding_locale(project: &Project, b: &revel_edit::canvas::Binding) -> Option<String> {
    use revel_edit::canvas::Source;
    match &b.source {
        Source::LocaleData { locale } => Some(locale.clone()),
        Source::Entry { folder, .. } | Source::EntryItem { folder, .. } => {
            let primary = project.primary_folder(folder)?;
            project
                .site
                .locales
                .iter()
                .find(|l| revel_model::locale_folder(&primary, &l.code) == *folder)
                .map(|l| l.code.clone())
        }
        _ => None,
    }
}

/// Commande appliquée par le cœur (elle écrit des fichiers du projet), ou suite qui en contient.
fn external(c: &Command) -> bool {
    match c {
        Command::Batch { commands } => commands.iter().any(external),
        Command::SetBound { .. }
        | Command::SetEntryField { .. }
        | Command::SetEntryBody { .. }
        | Command::CreateEntry { .. }
        | Command::WriteEntry { .. }
        | Command::SetCmsField { .. }
        | Command::AddCmsField { .. }
        | Command::RemoveCmsField { .. }
        | Command::MoveCmsField { .. }
        | Command::SetCmsCollection { .. }
        | Command::CreateCmsCollection { .. }
        | Command::RemoveCmsCollection { .. }
        | Command::WriteCmsConfig { .. }
        | Command::SetDataValue { .. }
        | Command::MoveFiles { .. }
        | Command::WriteDataFile { .. } => true,
        _ => false,
    }
}

/// Nouvelle collection : nom et dossier vérifiés, collection ajoutée au site et décrite dans
/// `.pages.yml` (créé au besoin avec `settings.content.merge`).
fn create_collection(
    project: &Project,
    site: &mut revel_model::Site,
    id: &str,
    label: &str,
    folder: &str,
    fields: &serde_json::Value,
) -> std::result::Result<Command, revel_edit::EditError> {
    use revel_edit::EditError::Bound;
    let ok_name = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    };
    let folder = crate::pagescms::norm_path(folder);
    if !ok_name(id) {
        return Err(Bound(format!("nom de collection refusé : {id}")));
    }
    if folder.is_empty()
        || folder.starts_with('/')
        || folder
            .split('/')
            .any(|p| p == "." || p == ".." || !ok_name(p))
    {
        return Err(Bound(format!("dossier refusé : {folder}")));
    }
    if site.collections.iter().any(|c| c.id == id) {
        return Err(Bound(format!("collection déjà présente : {id}")));
    }
    if site
        .collections
        .iter()
        .any(|c| crate::pagescms::norm_path(&c.folder) == folder)
    {
        return Err(Bound(format!("dossier déjà utilisé : {folder}")));
    }
    let old = pagesyml::read(&project.dir).map_err(Bound)?;
    let mut text = old.clone().unwrap_or_default();
    let created = text.trim().is_empty();
    let names: Vec<&str> = fields
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|f| f.get("name").and_then(|n| n.as_str()))
                .collect()
        })
        .unwrap_or_default();
    let mut def = serde_json::json!({
        "name": id,
        "label": label,
        "type": "collection",
        "path": folder,
        "filename": if names.contains(&"slug") { "{fields.slug}.md" } else { "{primary}.md" },
    });
    if let Some(p) = names.first() {
        def["view"] = serde_json::json!({ "primary": p });
    }
    def["fields"] = fields.clone();
    text = pagesyml::add_collection(&text, &def).map_err(Bound)?;
    if created {
        text = pagesyml::set_top(
            &text,
            "settings",
            &serde_json::json!({ "content": { "merge": true } }),
        )
        .map_err(Bound)?;
    }
    pagesyml::write(&project.dir, Some(&text)).map_err(Bound)?;
    site.collections.push(revel_model::Collection {
        id: id.to_string(),
        source: revel_model::CollectionSource::Folder,
        folder,
        path: None,
        layout: None,
        props: Default::default(),
        filter: None,
        sort: None,
        fields: vec![],
    });
    Ok(Command::RemoveCmsCollection {
        id: id.to_string(),
        config: old,
    })
}

fn object(
    v: &serde_json::Value,
) -> std::result::Result<serde_json::Map<String, serde_json::Value>, String> {
    v.as_object()
        .cloned()
        .ok_or_else(|| "modification attendue : objet".to_string())
}

/// En-têtes des contenus d'un dossier.
fn entries_data(project: &Project, folder: &str) -> Vec<serde_json::Value> {
    project
        .load_entries()
        .map(|list| {
            list.into_iter()
                .filter(|e| e.folder == folder)
                .map(|e| e.data)
                .collect()
        })
        .unwrap_or_default()
}

/// Nouveau champ sans type : type, libellé et options déduits des valeurs des contenus (valeur
/// présente dans les contenus mais absente du schéma).
fn complete_field(
    project: &Project,
    folder: &str,
    def: &serde_json::Value,
) -> std::result::Result<serde_json::Value, revel_edit::EditError> {
    if def.get("type").is_some() {
        return Ok(def.clone());
    }
    let name = def
        .get("name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| revel_edit::EditError::Bound("champ sans nom".into()))?;
    let data = entries_data(project, folder);
    let values: Vec<&serde_json::Value> = data.iter().filter_map(|d| d.get(name)).collect();
    let mut inferred = crate::pagescms::infer_field(name, &values, 0);
    if let (Some(o), Some(extra)) = (inferred.as_object_mut(), def.as_object()) {
        for (k, v) in extra {
            o.insert(k.clone(), v.clone());
        }
    }
    Ok(inferred)
}

/// Modification du schéma d'une collection dans `.pages.yml` : la collection y est d'abord décrite
/// d'après ses contenus si elle n'y est pas (fichier créé au besoin, avec
/// `settings.content.merge` pour que Pages CMS garde les valeurs non déclarées) ; inverse : texte
/// exact du fichier d'avant.
fn schema_edit(
    project: &Project,
    folder: &str,
    edit: impl FnOnce(&str, &str) -> std::result::Result<String, String>,
) -> std::result::Result<Command, revel_edit::EditError> {
    use revel_edit::EditError::Bound;
    let old = pagesyml::read(&project.dir).map_err(Bound)?;
    let mut text = old.clone().unwrap_or_default();
    let config = crate::pagescms::parse(&text);
    let path = crate::pagescms::norm_path(folder);
    let name = match config.collections.iter().find(|c| c.path == path) {
        Some(c) => c.name.clone(),
        None => {
            let c = project
                .site
                .collections
                .iter()
                .find(|c| !c.folder.is_empty() && c.folder == folder)
                .ok_or_else(|| Bound(format!("collection inconnue : {folder}")))?;
            // Nom unique parmi les contenus du fichier.
            let doc = revel_project::parse_yaml(&text).unwrap_or_default();
            let taken = |n: &str| {
                doc.get("content")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|l| {
                        l.iter()
                            .any(|x| x.get("name").and_then(|v| v.as_str()) == Some(n))
                    })
            };
            let mut name = c.id.clone();
            let mut n = 2;
            while taken(&name) {
                name = format!("{}-{n}", c.id);
                n += 1;
            }
            let data = entries_data(project, folder);
            let refs: Vec<&serde_json::Value> = data.iter().collect();
            let def = crate::pagescms::collection_def(
                &name,
                &crate::pagescms::humanize(&c.id),
                folder,
                &refs,
            );
            let created = text.trim().is_empty();
            text = pagesyml::add_collection(&text, &def).map_err(Bound)?;
            if created {
                text = pagesyml::set_top(
                    &text,
                    "settings",
                    &serde_json::json!({ "content": { "merge": true } }),
                )
                .map_err(Bound)?;
            }
            name
        }
    };
    let new = edit(&text, &name).map_err(Bound)?;
    pagesyml::write(&project.dir, Some(&new)).map_err(Bound)?;
    Ok(Command::WriteCmsConfig { text: old })
}

/// Écrit le fichier d'un contenu ; inverse : son texte d'avant (`None` : il n'existait pas).
fn write_entry(
    project: &Project,
    folder: &str,
    file: &str,
    text: Option<&str>,
    old: Option<String>,
) -> std::result::Result<Command, revel_edit::EditError> {
    project
        .write_entry(folder, file, text)
        .map_err(revel_edit::EditError::Bound)?;
    Ok(Command::WriteEntry {
        folder: folder.to_string(),
        file: file.to_string(),
        text: old,
    })
}

/// Empreinte des fichiers lus par le canevas (document, données, contenus) : change quand l'un
/// d'eux est modifié, ajouté ou retiré.
fn files_stamp(project: &Project) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(d) = &project.site.data_dir {
        dirs.push(project.dir.join(d));
    }
    for c in &project.site.collections {
        if !c.folder.is_empty() {
            dirs.push(project.dir.join(&c.folder));
            // Contenus des autres langues (3.8).
            for l in &project.site.locales {
                dirs.push(
                    project
                        .dir
                        .join(revel_model::locale_folder(&c.folder, &l.code)),
                );
            }
        }
    }
    dirs.sort();
    dirs.dedup();
    let mut files = vec![project.dir.join(revel_project::SITE_FILE)];
    fn walk(d: &Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && depth < 4 {
                walk(&p, out, depth + 1);
            } else {
                out.push(p);
            }
        }
    }
    for d in &dirs {
        walk(d, &mut files, 0);
    }
    files.sort();
    for f in files {
        f.hash(&mut h);
        if let Ok(m) = std::fs::metadata(&f) {
            m.len().hash(&mut h);
            if let Ok(t) = m.modified() {
                t.hash(&mut h);
            }
        }
    }
    format!("{:016x}", h.finish())
}

fn key(path: &Path) -> String {
    path.display().to_string()
}

impl Core {
    /// Projet d'un site ouvert pour l'édition (lecture).
    pub(crate) fn with_project<T>(
        &self,
        path: &Path,
        f: impl FnOnce(&Project) -> Result<T>,
    ) -> Result<T> {
        self.with_session(path, |s| f(&s.project))
    }

    /// Modifie le projet d'un site ouvert hors de l'historique (réglage technique, comme le
    /// dossier des médias déclaré au premier import) et l'enregistre.
    pub(crate) fn with_session_mut(
        &self,
        path: &Path,
        f: impl FnOnce(&mut Project) -> Result<()>,
    ) -> Result<()> {
        self.with_session(path, |s| {
            f(&mut s.project)?;
            s.save()
        })
    }

    fn with_session<T>(&self, path: &Path, f: impl FnOnce(&mut Session) -> Result<T>) -> Result<T> {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let s = map.get_mut(&key(path)).ok_or(CoreError::NotOpen)?;
        f(s)
    }

    /// Ouvre un site pour l'édition (ou reprend la session en cours). Si le fichier a été modifié
    /// hors de Revel depuis le dernier enregistrement, il est relu et l'historique repart de zéro.
    pub fn edit_open(&self, path: &Path) -> Result<Opened> {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let k = key(path);
        let stale = map
            .get(&k)
            .is_some_and(|s| s.saved_mtime.is_some() && s.saved_mtime != mtime(path));
        if stale || !map.contains_key(&k) {
            let project = Project::open(path)?;
            map.insert(
                k.clone(),
                Session {
                    project,
                    history: History::default(),
                    revision: 0,
                    saved_mtime: mtime(path),
                },
            );
        }
        let s = map.get(&k).ok_or(CoreError::NotOpen)?;
        Ok(Opened {
            site: s.project.site.clone(),
            state: s.state(None),
        })
    }

    /// Applique une commande, l'enregistre dans l'historique et sur disque.
    /// `coalesce` : saisie continue dans un même champ (une seule étape d'annulation).
    pub fn edit_apply(&self, path: &Path, cmd: Command, coalesce: bool) -> Result<EditState> {
        self.edit_apply_in(path, cmd, coalesce, None)
    }

    /// Comme [`Core::edit_apply`] ; les commandes d'un même `group` envoyées à la suite forment
    /// une seule étape d'annulation (traduction automatique faite par lots).
    pub fn edit_apply_grouped(&self, path: &Path, cmd: Command, group: &str) -> Result<EditState> {
        self.edit_apply_in(path, cmd, false, Some(group))
    }

    fn edit_apply_in(
        &self,
        path: &Path,
        cmd: Command,
        coalesce: bool,
        group: Option<&str>,
    ) -> Result<EditState> {
        self.with_session(path, |s| {
            // Texte lié saisi dans le canevas : sans la typographie de la page si la source ne
            // l'utilisait pas.
            let cmd = match cmd {
                Command::SetBound { binding, value }
                    if s.project.site.settings.typography.as_deref() == Some("fr")
                        && binding_locale(&s.project, &binding)
                            .is_none_or(|l| l.starts_with("fr")) =>
                {
                    let old = s.project.bound_value(&binding).unwrap_or_default();
                    Command::SetBound {
                        value: revel_edit::canvas::untypo(&value, &old),
                        binding,
                    }
                }
                other => other,
            };
            let project = &s.project;
            let mut site = project.site.clone();
            let mut run = |site: &mut revel_model::Site, c: &Command| exec(project, site, c);
            let applied = match group {
                Some(g) => s.history.apply_grouped(&mut site, cmd, g, &mut run),
                None => s.history.apply_with(&mut site, cmd, coalesce, &mut run),
            }
            .map_err(CoreError::Edit)?;
            s.project.site = site;
            s.revision += 1;
            s.save()?;
            Ok(s.state(Some(applied)))
        })
    }

    /// Empreinte des fichiers du site lus par le canevas : l'interface la compare régulièrement
    /// pour recharger la page quand un fichier a été modifié hors de Revel (Pages CMS…).
    pub fn edit_stamp(&self, path: &Path) -> Result<String> {
        self.with_project(path, |p| Ok(files_stamp(p)))
    }

    /// Termine une saisie continue (la prochaine commande ouvre une nouvelle étape).
    pub fn edit_seal(&self, path: &Path) -> Result<()> {
        self.with_session(path, |s| {
            s.history.seal();
            Ok(())
        })
    }

    pub fn edit_undo(&self, path: &Path) -> Result<EditState> {
        self.with_session(path, |s| {
            let project = &s.project;
            let mut site = project.site.clone();
            let applied = s
                .history
                .undo_with(&mut site, &mut |site, c| exec(project, site, c))
                .map_err(CoreError::Edit)?;
            s.project.site = site;
            if applied.is_some() {
                s.revision += 1;
                s.save()?;
            }
            Ok(s.state(applied))
        })
    }

    pub fn edit_redo(&self, path: &Path) -> Result<EditState> {
        self.with_session(path, |s| {
            let project = &s.project;
            let mut site = project.site.clone();
            let applied = s
                .history
                .redo_with(&mut site, &mut |site, c| exec(project, site, c))
                .map_err(CoreError::Edit)?;
            s.project.site = site;
            if applied.is_some() {
                s.revision += 1;
                s.save()?;
            }
            Ok(s.state(applied))
        })
    }

    /// Document en cours d'édition (copie).
    pub fn edit_site(&self, path: &Path) -> Result<revel_model::Site> {
        self.with_session(path, |s| Ok(s.project.site.clone()))
    }

    /// Relit depuis le disque le site d'une session ouverte (fichiers remplacés : version
    /// restaurée, récupérée) ; l'historique repart de zéro. Sans session : rien.
    pub fn edit_reload(&self, path: &Path) -> Result<()> {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(s) = map.get_mut(&key(path)) {
            s.project = Project::open(path)?;
            s.history = History::default();
            s.revision += 1;
            s.saved_mtime = mtime(path);
        }
        Ok(())
    }

    pub fn edit_close(&self, path: &Path) {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(&key(path));
    }
}
