//! `revel-mcp` : serveur MCP exposant l'édition à Claude (connecteur claude.ai) et tunnel.
//!
//! Interface posée à l'étape 0 ; implémentation prévue : Étape 4.

/// Outils prévus pour le connecteur Claude. Chaque outil sera déclaré au
/// serveur MCP ; la liste sert de contrat entre l'éditeur et Claude.
pub const PLANNED_TOOLS: &[&str] = &[
    "list_pages",
    "get_page",
    "update_element",
    "add_section",
    "list_styles",
    "update_style",
    "preview",
    "publish_draft",
];

#[cfg(test)]
mod tests {
    #[test]
    fn noms_uniques() {
        let mut v = super::PLANNED_TOOLS.to_vec();
        v.sort();
        v.dedup();
        assert_eq!(v.len(), super::PLANNED_TOOLS.len());
    }
}
