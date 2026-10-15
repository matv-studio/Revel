# Étape 3 — Contenus, pages, composants, médias, langues

Objectif : construire et faire vivre un site entier dans Revel, sans fichier à toucher à la main — pages,
contenus du CMS (avec leurs champs et leurs textes riches), médias, composants réutilisables, blocs prêts à
l'emploi, langues, réglages du site — en gardant les fichiers lisibles par Pages CMS et l'équivalence de la
sortie (étapes 1 et 2).

**Critères d'acceptation**
1. Une page se crée, se renomme (adresse), se duplique, se met en brouillon et se supprime depuis Revel ; son
   titre, sa description et son image de partage se règlent dans ses paramètres.
2. Un contenu de collection se crée, se modifie (chaque type de champ, texte riche compris) et se supprime
   depuis la vue CMS ; le fichier Markdown produit est lisible par Pages CMS ; annuler rend le fichier identique
   octet pour octet.
3. Les champs d'une collection se modifient (ajout, retrait, type, libellé) ; `.pages.yml` suit.
4. Une image s'importe (fichier ou glisser depuis le système) et se place dans la page ou dans un champ image.
5. Une sélection devient un composant ; ses instances suivent ses modifications ; ses propriétés (texte, image,
   lien) se règlent par instance.
6. Des blocs prêts à l'emploi (sections, grilles, colonnes, liste de collection) s'ajoutent par glisser-déposer.
7. Une seconde langue s'ajoute ; chaque page et chaque contenu se traduit ; adresses par langue et balises
   `hreflang` générées.
8. Les modifications passent toutes par des commandes (D-13) : annulables, et utilisables par le serveur MCP de
   l'étape 4.

## Sous-étapes

| N | Contenu | Modules |
|---|---|---|
| 3.1 | **Édition des contenus** : commandes appliquées par le cœur (champ, texte, création, suppression ; inverse = texte exact du fichier), formulaires par type de champ (texte, texte long, texte riche, image, date, oui/non, nombre, liste, référence), brouillons, recherche et tri | `revel-edit`, `revel-project`, `revel-core`, `revel-ui` |
| 3.2 | **Texte riche** : éditeur visuel (titres, gras, italique, liens, listes, citations, images) qui écrit du Markdown, dans la vue CMS et sur la page | `revel-ui`, `revel-render` |
| 3.3 | **Champs des collections** : ajout, retrait, ordre, type, libellé ; nouvelle collection ; `.pages.yml` (Pages CMS) tenu à jour | `revel-edit`, `revel-cms`, `revel-ui` |
| 3.4 | **Pages** : nouvelle page, dossier, renommer, adresse, dupliquer, brouillon, supprimer ; paramètres de page (titre, description, image de partage, indexation) ; référencement du site (plan du site, robots.txt) | `revel-edit`, `revel-render`, `revel-ui` |
| 3.5 | **Médias** : import (sélecteur, glisser depuis le système), dossier des médias, remplacement, texte alternatif par défaut, choix d'une image pour un champ ou un élément | `revel-assets`, `revel-core`, `revel-ui` |
| 3.6 | **Composants** : créer depuis la sélection, instances, modification isolée, propriétés par instance (texte, image, lien, visibilité), détacher ; panneau Composants | `revel-model`, `revel-edit`, `revel-render`, `revel-ui` |
| 3.7 | **Blocs et éléments** : grille, colonnes, vidéo, icône (SVG), code HTML, liste de collection (boucle liée à une collection, filtre, tri, limite) ; blocs prêts à l'emploi ; modèles de site pour un nouveau site | `revel-edit`, `revel-ui` |
| 3.8 | **Langues** : langues du site, contenus et textes par langue, adresses par langue, sélecteur de langue, `hreflang` ; vue Localisation (tableau des textes à traduire, état par langue) | `revel-model`, `revel-render`, `revel-ui` |
| 3.9 | **Réglages du site** : nom, adresse, langue modifiables ; polices (fichiers, Google Fonts téléchargées dans le site) ; code ajouté (en-tête, fin de page, par site ou par page) | `revel-model`, `revel-render`, `revel-ui` |
| 3.10 | **Versions et synchronisation** : sauvegardes (versions du site comparées et restaurées), synchronisation avec le dépôt GitHub du site (enregistrements de Pages CMS) | `revel-git`, `revel-core`, `revel-ui` |
| 3.11 | **Canevas, suite** : poignées de taille (largeur et hauteur écrites dans la classe), marges visibles au survol, plusieurs points de rupture côte à côte, sélection multiple, palette de commandes (Ctrl+K) | `revel-core` (canvas.js), `revel-ui` |

## Choix retenus (15/10/2026, voir DECISIONS.md)

- **D-23 — langues** : un dossier de contenus et de données par langue, adresses préfixées (`/en/…`), comme
  Webflow Localization.
- **D-24 — polices** : fichiers copiés dans le site (aucune requête vers Google).
- **D-25 — synchronisation** : enregistrement et envoi sur le dépôt GitHub du site à la publication, ou à la
  demande.
- **D-26 — aperçu et export** : aperçu sans génération préalable ; « Générer » devient « Exporter ».

## Avancement

| N | État |
|---|---|
| 3.1 | fait : commandes `SetEntryField`, `SetEntryBody`, `CreateEntry`, `WriteEntry` appliquées par le cœur (`revel-project::entries` : champ simple réécrit sur place avec son style, champ composé réécrit en bloc à sa place, champ ajouté en fin d'en-tête, dates ISO sans guillemets, relecture avant écriture ; inverse = texte exact du fichier) ; vue CMS : fiche modifiable (texte, texte long, date, oui/non, nombre, liste en pastilles, image, objet en JSON), texte du contenu, nouveau contenu (nom de fichier tiré du titre), suppression confirmée, annuler/rétablir (boutons, Ctrl+Z) dans l'historique du site ; l'éditeur relit pages et canevas. Essais : 4 essais d'écriture YAML, 1 essai du cœur (tout annulé octet pour octet, chemins refusés) ; essais de l'interface 97/97 |
| 3.2 | fait : éditeur de texte riche (`richtext.rs`) : paragraphes, titres 2 à 4, citation, gras, italique, liens (Ctrl+K), listes à puces et numérotées, retrait de la mise en forme, collage en texte brut, source Markdown ou HTML ; écrit en Markdown (texte du contenu) ou en HTML nettoyé (champs importés de WordPress : liste blanche de balises, sans styles), réécrit seulement s'il a changé ; texte sur plusieurs lignes gardé en bloc littéral dans l'en-tête. Essais de l'interface : 98/98 |
