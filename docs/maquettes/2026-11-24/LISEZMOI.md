# Maquettes validées, pas encore codées (Revel, 24/11/2026)

Source : décisions D-197 et D-198 de `docs/DECISIONS.md` (dépôt `matv-studio/Revel`). Captures en anglais,
thème sombre. La maquette fait foi, avec les ajustements ci-dessous (validés par l'utilisateur le 24/11 à
14 h 02).

## 1. Bibliothèques d'icônes (étape suivante)

- `icons-1-styles-closed.png`, `icons-1-styles-open.png` : panneau Styles d'une icône, menu « Library » avec
  « All » en tête (nombre d'icônes par bibliothèque) et « Manage libraries… » en bas, avec l'icône de rouage
  de l'application (`icons::settings`, règle : tout ce qui ouvre des réglages porte le rouage).
- `icons-2-settings.png` : Réglages de Revel › « Icon libraries », sur le modèle des catalogues de templates :
  une ligne par bibliothèque (nom, dépôt GitHub, dossier des SVG, licence, nombre d'icônes, retrait), ajout
  d'un dépôt `owner/repository` + dossier des SVG (facultatif).
- **Ajustements validés** (D-198) :
  - téléchargement : **la liste des noms** d'abord, puis **chaque dessin à l'affichage** (pas toute la
    bibliothèque d'un coup) ; dessins gardés en cache ;
  - **Revel et Lucide actives au départ** ; Tabler, Phosphor, Heroicons, Feather proposées et ajoutées **en un
    clic** ;
  - une icône choisie est **copiée dans le site** (le site ne dépend pas de la bibliothèque en ligne).
- Licences vérifiées le 24/11 : Lucide ISC, Tabler MIT, Phosphor MIT, Heroicons MIT, Feather MIT. Remix Icon
  écartée par défaut (statut de licence signalé comme incertain par Fedora).

## 2. Imports (étape d'après)

- `import-1-pages-revel-site.png` : « Import pages… » depuis un autre site Revel (choix des pages).
- `import-2-pages-html.png` : « Import pages… » depuis des fichiers HTML.
- `import-3-component.png` : « Import component… » depuis un autre site Revel, un fichier de composant ou un
  fichier HTML.
- Plus « Export component… » dans le menu d'un composant.
