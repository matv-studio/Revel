# Modèles de traduction

Dossier inclus dans les installateurs (ressources Tauri) : modèles de traduction automatique hors
ligne livrés avec Revel (D-36), français ↔ anglais. Les autres langues se téléchargent depuis les
réglages de l'application (catalogue : `crates/revel-translate/src/catalog.rs`).

Les modèles ne sont pas dans le dépôt (environ 150 Mo chacun). Pour les préparer ici :

    cargo run --release -p revel-translate --features download --example preparer -- apps/revel-app/models fr

La construction des installateurs le fait d'elle-même (`.github/workflows/release.yml`). Sans
modèle, Revel fonctionne normalement ; seule la traduction automatique est indisponible.
