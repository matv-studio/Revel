# Contrôle de l'interface (D-169)

Règles de la charte (`docs/DESIGN.md`) mesurées dans la page, écran par écran.

- `audit-page.js` : contrôles exécutés dans la page (libellé et info-bulle des boutons-icônes, curseur,
  cibles de clic, nom accessible des champs, hauteurs et tailles de texte hors échelle, capitales, phrase
  visible, clé de traduction visible, contraste AA, défilement horizontal, rôle des choix exclusifs).
- `lib.mjs` : ouverture de l'interface (Playwright), capture, parcours d'une zone défilante, ouverture puis
  fermeture d'un menu par Échap (un menu resté ouvert est signalé).
- `parcours-*.mjs` : écrans, menus et fenêtres visités ; résultats fusionnés dans `audit.json`.
- `essai-fermeture-menus.mjs` : chaque menu fermé par Échap, clic dans le canevas, clic dans un panneau.
- `essai-styles.mjs <sélecteur>` : chaque commande du panneau Styles (effet dans le canevas, temps de
  réaction, annulation).
- `mesures.mjs` : temps de chargement, de changement de vue, de sélection, d'annulation ; tâches longues.
- `report.py audit.json [règle] [limite]` : bilan par règle et par sélecteur.

Lancement : interface construite servie (`trunk build --release`, `python3 -m http.server 8171` dans
`apps/revel-ui/dist`), site d'essai servi par `revel canvas-serve <site> --port 8162 --bridge`, puis
`REVEL_AUDIT_OUT=sortie node tools/ui-audit/parcours-editeur.mjs`. Variables : `REVEL_UI`, `REVEL_BRIDGE`,
`REVEL_AUDIT_OUT`. Les parcours visent un site qui a des collections, plusieurs pages et des menus.
