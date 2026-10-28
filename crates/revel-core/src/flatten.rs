//! Styles mis à plat dans un projet (D-157) : pages rendues pour l'éditeur, puis
//! [`revel_import::flatten`].

use revel_edit::Command;
use revel_import::flatten::Report;
use revel_project::Project;

/// Contenus rendus par collection : assez pour voir un élément répété à plusieurs endroits.
const PER_SECTION: usize = 3;

/// Commandes de la mise à plat (une suite) et bilan.
pub fn plan(p: &Project) -> Result<(Command, Report), String> {
    let pages = p.editor_pages(PER_SECTION).map_err(|e| e.to_string())?;
    let components = p.editor_component_pages().map_err(|e| e.to_string())?;
    let content = p.content_html().map_err(|e| e.to_string())?;
    let (commands, report) =
        revel_import::flatten::flatten_with(&p.site, &pages, &components, &content);
    Ok((Command::Batch { commands }, report))
}

/// Met à plat les styles d'un projet et l'enregistre. Rend le bilan en une phrase (ou l'erreur).
pub fn flatten_project(p: &mut Project) -> Result<Report, String> {
    // Identifiants des nœuds (repères des pages rendues) : un projet tout juste créé n'en a pas.
    p.site.ensure_ids();
    let (cmd, report) = plan(p)?;
    if matches!(&cmd, Command::Batch { commands } if commands.is_empty()) {
        return Ok(report);
    }
    revel_edit::apply(&mut p.site, &cmd).map_err(|e| e.to_string())?;
    p.save().map_err(|e| e.to_string())?;
    Ok(report)
}

impl crate::Core {
    /// Mise à plat des styles du site ouvert, sans l'appliquer : l'interface (Classes › Rendre
    /// réglable) et les assistants l'appliquent comme toute commande (une étape d'annulation).
    pub fn edit_flatten_plan(
        &self,
        path: &std::path::Path,
    ) -> crate::Result<revel_edit::styleaudit::FlattenPlan> {
        let (command, report) =
            self.with_project(path, |p| plan(p).map_err(crate::CoreError::Other))?;
        Ok(revel_edit::styleaudit::FlattenPlan { command, report })
    }
}

/// Bilan en une phrase (avertissement d'import).
pub fn summary(r: &Report) -> Option<String> {
    if r.flattened + r.dead + r.translated + r.dropped == 0 && r.kept.is_empty() {
        return None;
    }
    Some(format!(
        "styles mis à plat : {} règles reportées sur les classes ({} classes ajoutées, {} pseudo-éléments devenus des éléments), {} règles sans élément retirées, {} déclarations traduites, {} gardées telles quelles (liste : revel flatten --dry-run)",
        r.flattened,
        r.classes,
        r.pseudo,
        r.dead,
        r.translated,
        r.kept.len()
    ))
}
