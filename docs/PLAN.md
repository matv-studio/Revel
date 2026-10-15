# Plan de Revel

Chaque étape se termine par une étiquette Git (`step-N`) et des installateurs de test ; le détail de l'étape en
cours est dans `ETAPE-N.md`, les choix dans `DECISIONS.md`.

| Étape | Contenu | État |
|---|---|---|
| 0 | Squelette modulaire (crates), intégration continue sur Windows, macOS et Linux | terminée |
| 1 | Projet sur disque, import d'un projet Eleventy, génération (images AVIF/WebP), aperçu, publication SFTP et GitHub Pages, interface français/anglais | terminée |
| 2 | Éditeur visuel : canevas, calques, réglages, panneau de styles, points de rupture, textes modifiés sur place, espace de travail modulable, glisser-déposer, vues du site (Éditeur, CMS, Localisation, Paramètres) | terminée (0.2.2) |
| 3 | Contenus du CMS (champs, texte riche, collections), pages, médias, composants, blocs, langues (un dossier par langue), réglages du site (polices copiées dans le site, code ajouté, référencement), sauvegardes, synchronisation GitHub à la publication, suite du canevas | en cours (`ETAPE-3.md`) |
| 4 | Assistants IA dans Revel, quel que soit le modèle (D-31) : serveur MCP (`revel-mcp`) en local (stdio et HTTP sur la machine : Claude Desktop, Claude Code, ChatGPT de bureau, Codex, LM Studio, Open WebUI avec Ollama…), accès à distance par tunnel sécurisé (connecteurs de ChatGPT et de claude.ai), interface HTTP décrite en OpenAPI pour les outils sans MCP ; plus tard, discussion intégrée avec le fournisseur choisi (Anthropic, OpenAI, modèle local compatible OpenAI) | à venir |
| 5 | Noms de domaine (OVH, Cloudflare), autres hébergeurs, réception des formulaires | à venir |
| 6 | Animations et interactions (dont animations Lottie exportées d'After Effects), empaquetage, mises à jour automatiques, signature du code, documentation | à venir |
| 7 | Import depuis d'autres outils, le plus fidèle possible (D-33) : site HTML statique (dossier ou adresse), Webflow (export du code et contenus du CMS par son API), Framer (site publié lu à son adresse, pas d'export proposé par Framer), Figma (maquettes lues par son API : cadres, composants, variables, images), WordPress (contenus, médias, redirections), en plus d'Eleventy | à venir (ajoutée le 15/10/2026) |
