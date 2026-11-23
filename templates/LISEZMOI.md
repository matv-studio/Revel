# Modèles de site de Revel

Ce dossier est le catalogue des modèles proposés par « Nouveau site ». Revel le lit en ligne
(`raw.githubusercontent.com/matv-studio/Revel/HEAD/templates/`) et en garde une copie sur l'ordinateur : un
modèle ajouté, changé ou retiré ici arrive chez les utilisateurs sans nouvelle version de Revel (D-194).

- Un modèle = un dossier `<identifiant>/` (`template.json`, `site.css`, `pages/`, `components/`, `layouts/`,
  `files/`). `_kit/` : composants et styles communs. `photos.json` : photos des modèles.
- Textes en deux langues : `[[français||English]]`.
- **Après tout changement** : `revel templates-index templates` refait `index.json` (liste des fichiers et
  révision). Sans cela l'application ne voit pas le changement ; un essai de `revel-core` le contrôle.
- Retirer un modèle : supprimer son dossier, refaire l'index.
- Essayer un dossier local : variable d'environnement `REVEL_TEMPLATES_DIR`.
