# Étape 8 — Fonctions optionnelles

Fonctions utiles mais non nécessaires au fonctionnement de Revel, faites quand l'utilisateur le décide
(demande du 17/10/2026).

## Sous-étapes

| N | Contenu | État |
|---|---|---|
| 8.1 | **Signature du code** Windows et macOS (ancienne 6.7, D-09) : plus d'avertissement de sécurité à l'installation. Construction prête : signature Windows par Azure Artifact Signing (secrets `AZURE_*`, variables `ARTIFACT_SIGNING_*`), macOS par certificat Developer ID et notarisation (secrets `APPLE_*`) ; sans eux, installateurs non signés et signature ad hoc sous macOS. Coûts et voies gratuites étudiés dans D-09 | Optionnelle, en attente du choix de l'utilisateur |
