# Étape 4 — Assistants IA dans Revel

Objectif : piloter Revel depuis n'importe quel assistant (Claude, ChatGPT, Codex, modèle local) par le
protocole MCP (D-13, D-31), avec les mêmes commandes que l'interface : chaque modification faite par un
assistant est visible en direct dans Revel, annulable et présente dans l'historique.

**Critères d'acceptation**
1. Un assistant local (Claude Desktop, Claude Code, Codex et ChatGPT de bureau, LM Studio) se branche sur Revel
   par une commande (`revel --mcp`, stdio) copiée depuis les réglages ; Open WebUI par une adresse HTTP locale
   avec jeton.
2. L'assistant lit le site ouvert (pages, structure d'une page, styles, contenus du CMS, langues) et le modifie
   (textes, éléments, styles, contenus, pages) ; Revel affiche la modification aussitôt ; Ctrl+Z l'annule.
3. Rien n'écoute hors de l'ordinateur sans action de l'utilisateur : serveur HTTP sur 127.0.0.1, désactivé par
   défaut, jeton exigé, hôtes et origines contrôlés.
4. Les outils sont décrits en clair (français et anglais), leurs paramètres par un schéma JSON ; les erreurs
   reviennent lisibles (même texte que dans l'interface).
5. Les mêmes opérations sont décrites en OpenAPI pour les outils sans MCP.

## Sous-étapes

| N | Contenu | Modules |
|---|---|---|
| 4.1 | **Serveur MCP** (D-41) : outils de lecture et d'écriture définis une fois (`revel-mcp`, SDK officiel `rmcp`), branchés soit sur le cœur (dans l'application et la ligne de commande), soit sur l'application ouverte (relais stdio de `revel --mcp`, lancée au besoin) ; HTTP en flux sur 127.0.0.1 avec jeton ; essais de bout en bout (protocole 2025 et 2026-07-28) | `revel-mcp`, `revel-core`, `revel-app`, `revel-cli` |
| 4.2 | **Réglages › Assistants IA** : activer le serveur, port, jeton (copier, renouveler), configurations prêtes à copier (Claude Desktop, Claude Code, Codex et ChatGPT, LM Studio, Open WebUI), dernières actions des assistants | `revel-ui`, `revel-app` |
| 4.3 | **OpenAPI** : mêmes opérations en HTTP simple, description OpenAPI servie et téléchargeable | `revel-mcp` |
| 4.4 | **Accès à distance** (connecteurs claude.ai et ChatGPT) : tunnel choisi par l'utilisateur, authentification OAuth, désactivé par défaut (recherche préalable des exigences des connecteurs) | `revel-mcp`, `revel-ui` |
| 4.5 | **Discussion dans Revel** (D-31, à décider) : panneau de discussion avec le fournisseur choisi (Anthropic, OpenAI, modèle local compatible OpenAI) utilisant les mêmes outils | `revel-ui`, `revel-mcp` |

## Avancement

| N | État |
|---|---|
| 4.1 | fait (D-41) : `revel-mcp` (SDK officiel `rmcp` 3.5) : 18 outils — lecture (`get_site`, `get_page` : plan de la page avec gabarits emboîtés et identifiants, `get_node`, `get_styles`, `get_page_html`, `get_data`, `list_collections`, `get_entry`) et écriture (`set_text`, `set_attributes`, `set_styles` par classe, point de rupture et état, `insert_html` en Nunjucks, `remove_node`, `set_data`, `set_entry_field`, `apply_commands`, `undo`, `redo`), chacun une étape d'annulation de Revel ; erreurs rendues en clair (texte de l'interface). Transports : stdio et HTTP en flux sur 127.0.0.1 (protocoles 2025 avec session et 2026-07-28 sans état), clé exigée, en-têtes `Origin` et `Host` contrôlés. Application : serveur démarré si activé (`mcp.json` des réglages, désactivé par défaut, port 47520), relais `revel --mcp` qui transmet chaque outil à l'application ouverte (lancée au besoin) ; ligne de commande : `revel mcp <site> [--http PORT]`, `canvas-serve --mcp-port`. Éditeur : document relu dès que la révision du cœur change (autre vue, assistant), relevé chaque seconde et au retour dans l'éditeur. Essais : 2 de `revel-mcp` (bout en bout HTTP : clé, origine, hôte, outils, annulation, protocole 2026-07-28 ; relais : clé fausse, serveur arrêté puis redémarré sur le même port, serveur désactivé), essais de l'interface 218/218 dont un assistant qui modifie un texte et une donnée, affichés aussitôt, annulés depuis l'éditeur |
