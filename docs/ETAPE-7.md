# Étape 7 — Import depuis d'autres outils

Objectif : reprendre dans Revel un site fait ailleurs, le plus fidèlement possible, pour le modifier ensuite
dans l'éditeur comme un site créé dans Revel (D-33). Toutes les sources passent par un même importateur de
HTML statique (7.1) : un export Webflow, un site publié par Framer ou un dépôt GitHub de pages HTML sont, une
fois lus, des pages HTML, des feuilles de style et des fichiers.

**Critères d'acceptation**
1. Un dossier de pages HTML (fait à la main, exporté d'un outil, produit par un générateur) s'ouvre dans
   l'éditeur : chaque page à sa même adresse, l'en-tête et le pied communs dans un gabarit, les styles
   modifiables dans le panneau Styles, les images dans les médias, les scripts gardés. Le site produit par
   Revel s'affiche comme l'original.
2. Un dépôt Git (adresse HTTPS, ou dépôt du compte GitHub connecté) s'importe sans installer Git : copie dans
   le dossier des projets, historique gardé (panneau Versions), puis import comme un dossier. Un site à
   construire (SvelteKit, Next, Astro, Vite…) est signalé avec la marche à suivre (dossier produit par sa
   construction), sans être construit par Revel (D-80).
3. Rien n'est modifié dans les fichiers d'origine : l'import ajoute `site.revel.json` et le dossier `.revel/`.
4. Ce qui n'est pas repris tel quel est dit à la fin de l'import (avertissements), sans bloquer.
5. Webflow, site publié (Framer et autres), Figma, WordPress : sources propres à chaque outil, puis
   compatibilité des animations et interactions.

## Sous-étapes

| N | Contenu | Modules |
|---|---|---|
| 7.1 | **Site HTML statique depuis un dossier** : pages (adresses gardées), gabarit commun (en-tête de document, début et fin du corps partagés par toutes les pages), titre et description dans le référencement de chaque page, feuilles de style locales fusionnées dans la feuille du site (D-18), fichiers copiés tels quels, scripts gardés, attributs propres à une page (`<body class>`) | `revel-import`, `revel-core`, `revel-ui` |
| 7.2 | **Depuis un dépôt Git** (option 1 du 17/10/2026) : adresse HTTPS d'un dépôt public, ou dépôt privé avec la connexion GitHub ; copie dans le dossier des projets, puis import Eleventy ou HTML statique ; site à construire signalé | `revel-git`, `revel-core`, `revel-ui` |
| 7.3 | **Webflow** : export du code (offres Workspace), décompressé par l'utilisateur, importé comme un dossier (7.1) : feuilles `normalize.css` et `webflow.css` comprises, formulaires repris en formulaires Revel ; contenus du CMS par l'API de Webflow (jeton) ou les fichiers CSV. Pas d'outil pour le fichier .zip (décision de l'utilisateur, 17/10/2026) | `revel-import` |
| 7.4 | **Import depuis un lien : site publié lu à son adresse** (Framer, qui n'exporte pas de HTML, et tout site en ligne) : pages suivies depuis l'accueil et le plan du site, fichiers téléchargés (images et polices des serveurs de l'outil comprises), puis 7.1 | `revel-import`, `revel-core` |
| 7.5 | **Figma** : fichier lu par l'API (cadres en sections, mise en page automatique en flex, variables en couleurs et tailles du site, images) | `revel-import` |
| 7.6 | **WordPress** : fichier d'export WXR (Outils › Exporter) : pages, articles en collection, médias, redirections | `revel-import`, `revel-cms` |
| 7.7 | **Animations et interactions** propres à chaque outil (interactions Webflow, effets de Framer) : reprises en animations Revel quand elles correspondent, sinon scripts gardés | `revel-import`, `revel-render` |
| 7.8 | **Passe de vérification** sur des sites réels : rendu de l'original et du site produit par Revel comparés page par page | outils d'essai |

## État

| N | État |
|---|---|
| 7.1 | Terminée (17/10/2026) : `revel_import::html`, boîte « Importer › Dossier », commande `revel import` ; fidélité mesurée par `revel import-check` (structure de chaque page produite comparée à l'originale) |
| 7.2 | Terminée (17/10/2026) : `revel_git::clone`, boîte « Importer › Dépôt Git » (adresse d'un dépôt, d'une branche ou d'un dossier, avancement, annulation), site à construire et licence absente signalés, dossier produit importable, copie abandonnée retirée (D-80) |
| 7.3 | À venir |
| 7.4 | À venir |
| 7.5 | À venir |
| 7.6 | À venir |
| 7.7 | À venir |
| 7.8 | À venir |

## Règles de l'import HTML statique (7.1)

- **Pages** : chaque fichier `.html` (hors `node_modules`, dossiers cachés) devient une page à la même adresse :
  `index.html` → `/`, `agence/index.html` → `/agence/`, `contact.html` → `/contact.html`. Les liens entre
  pages restent donc valides.
- **Gabarit** : éléments de l'en-tête présents à l'identique dans toutes les pages ; début et fin du corps
  communs à toutes les pages (navigation, pied de page), en descendant dans une enveloppe commune
  (`<div class="wrapper">`). Le reste du corps est le contenu de chaque page. Une seule page : le gabarit ne
  garde que l'en-tête.
- **Référencement** : `<title>` et `<meta name="description">` deviennent le titre et la description de la
  page (Paramètres de la page) ; les autres éléments d'en-tête propres à une page vont dans son code ajouté
  (en-tête), sauf les feuilles de style et les blocs `<style>`, placés au début du contenu de la page pour
  rester visibles dans le canevas. Les scripts propres à une page en fin de corps vont dans son code ajouté
  (fin du corps).
- **Attributs de `<html>` et `<body>`** : identiques partout, écrits dans le gabarit ; sinon variables de page.
- **Styles** : feuilles locales liées par toutes les pages, fusionnées dans l'ordre dans la feuille du site
  (adresse de la première), adresses `url()` réécrites depuis la racine ; feuille illisible par Revel : copiée
  telle quelle (avertissement). Feuilles externes (polices, bibliothèques) : liens gardés.
- **Fichiers** : tout le reste est copié tel quel à la même adresse (images, scripts, polices, `robots.txt`,
  `.htaccess`, `CNAME`) ; pas les fichiers de dépôt (`README`, `LICENSE`, `package.json`, `.git`, `.github`).
- **Texte** : `{{` et `{%` d'une page HTML restent du texte (pas d'expression Revel).

## Mesures (17/10/2026)

| Site | Pages | Résultat |
|---|---|---|
| Site Alunites produit par Revel (HTML statique, 66 pages, 442 fichiers) | 66 | 66/66 pages équivalentes ; 2 gabarits (en-tête, navigation avec le lien de la page en cours, pied communs), 395 règles dans le panneau Styles, import en 0,2 s ; styles calculés et captures identiques sur 9 pages à 1280 et 375 px (`tools/compare-styles.mjs`, 52 760 styles, 0 écart), captures pleine page des 66 pages identiques sauf images chargées à des moments différents (même écart entre deux captures de l'original) |
| Recréation d'Alunites produite par Revel | 64 | 64/64 pages équivalentes |
| Essais : pages à des profondeurs différentes, page sans navigation (404), feuilles propres à une page, scripts de fin de page, HTML peu soigné (balises non fermées, majuscules, entités) | 5 + 2 | Pages équivalentes à l'original (essais `html_import`, `depot_git_de_pages_html_importe`) |

Vérification ajoutée à l'intégration continue (tâche « Équivalence avec Eleventy ») : le site produit par Revel
est importé comme un dossier de pages HTML, puis comparé page par page (structure, styles calculés, captures).

Écart trouvé par la comparaison des captures et corrigé : le type de document (`<!DOCTYPE html>`) n'était pas
repris, ce qui faisait passer les pages en mode de compatibilité (hauteurs de ligne différentes).

## Framer : voies étudiées (17/10/2026)

Framer ne propose pas d'export HTML ni d'hébergement ailleurs que chez lui (article d'aide mis à jour le
15/09/2026 : https://www.framer.com/help/articles/can-i-export-my-website-to-html-and-self-host-it/).

| Voie | Ce qu'elle donne | Limites |
|---|---|---|
| Site publié lu à son adresse (7.4) | Pages déjà calculées par Framer (HTML complet), styles, images et polices téléchargés ; marche aussi pour tout site en ligne | Classes produites par Framer (`framer-abc123`) ; animations liées au moteur de Framer (scripts gardés ou remplacés par les animations de Revel en 7.7) ; formulaires à reprendre ; contenus du CMS lus comme des pages |
| Module (plugin) Framer ou API serveur de Framer (bêta ouverte depuis le 12/02/2026 : https://www.framer.com/updates/server-api) | Calques, textes, composants et collections du CMS lus dans le projet (https://framer.com/developers/nodes) | Module à écrire et à faire accepter par Framer ; tout n'est pas lisible (animations, effets) ; API en bêta, susceptible de changer |
