# Étape 8 — Fonctions optionnelles

Fonctions utiles mais non nécessaires au fonctionnement de Revel, faites quand l'utilisateur le décide
(demande du 17/10/2026).

## Sous-étapes

| N | Contenu | État |
|---|---|---|
| 8.1 | **Signature du code** Windows et macOS (ancienne 6.7, D-09) : plus d'avertissement de sécurité à l'installation. Construction prête : signature Windows par Azure Artifact Signing (secrets `AZURE_*`, variables `ARTIFACT_SIGNING_*`), macOS par certificat Developer ID et notarisation (secrets `APPLE_*`) ; sans eux, installateurs non signés et signature ad hoc sous macOS. Coûts et voies gratuites étudiés dans D-09 | Optionnelle, en attente du choix de l'utilisateur |
| 8.2 | **Plan du site nodal** (demande du 19/10/2026) : onglet avant les Paramètres, pages, collections, menus et parties communes reliés par leurs liens sur un plan infini ; nœuds déplacés gardés, liens changés de destination ou ajoutés en tirant un trait, pages créées, renommées, redirigées depuis le plan ; outil `get_site_map` des assistants (D-103) | Faite (19/10/2026) |
| 8.3 | **Paramètres réorganisés et corrections de l'interface** (demande du 19/10/2026) : rail et onglet raccordé à la page, parties en deux colonnes (titre et présentation, réglages) pour le site et Revel ; code affiché à côté des informations du site corrigé ; calque masqué caché dans le canevas ; champs de média seuls proposés au média au survol (D-104, D-105) | Faite (19/10/2026) |
