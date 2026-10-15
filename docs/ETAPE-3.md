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
| 3.3 | **Champs des collections** : schéma lu dans `.pages.yml` (Pages CMS) — libellés, colonnes, tri, contrôles, champs obligatoires, motifs, choix, références, objets ; puis ajout, retrait, ordre, type, libellé, nouvelle collection, écrits dans `.pages.yml` | `revel-core`, `revel-edit`, `revel-ui` |
| 3.4 | **Pages** : nouvelle page, dossier, renommer, adresse, dupliquer, brouillon, supprimer ; paramètres de page (titre, description, image de partage, indexation) ; référencement du site (plan du site, robots.txt) | `revel-edit`, `revel-render`, `revel-ui` |
| 3.5 | **Médias** : import (sélecteur, glisser depuis le système), dossier des médias, remplacement, texte alternatif par défaut, choix d'une image pour un champ ou un élément | `revel-assets`, `revel-core`, `revel-ui` |
| 3.6 | **Composants** : créer depuis la sélection, instances, modification isolée, propriétés par instance (texte, image, lien, visibilité), détacher ; panneau Composants | `revel-model`, `revel-edit`, `revel-render`, `revel-ui` |
| 3.7 | **Blocs et éléments** : grille, colonnes, vidéo, icône (SVG), code HTML, liste de collection (boucle liée à une collection, filtre, tri, limite) ; blocs prêts à l'emploi ; modèles de site pour un nouveau site | `revel-edit`, `revel-ui` |
| 3.8 | **Langues** : langues du site, contenus et textes par langue, adresses par langue, sélecteur de langue, `hreflang` ; vue Localisation (tableau des textes à traduire, état par langue) | `revel-model`, `revel-render`, `revel-ui` |
| 3.9 | **Réglages du site** : nom, adresse, langue modifiables ; polices (fichiers, Google Fonts téléchargées dans le site) ; code ajouté (en-tête, fin de page, par site ou par page) | `revel-model`, `revel-render`, `revel-ui` |
| 3.10 | **Versions et synchronisation** : sauvegardes (versions du site comparées et restaurées), synchronisation avec le dépôt GitHub du site (enregistrements de Pages CMS) | `revel-git`, `revel-core`, `revel-ui` |
| 3.11 | **Canevas, suite** : poignées de taille (largeur et hauteur écrites dans la classe), marges visibles au survol, plusieurs points de rupture côte à côte, sélection multiple, palette de commandes (Ctrl+K) | `revel-core` (canvas.js), `revel-ui` |
| 3.12 | **Réglages lisibles à la place des variables du code importé** (demande du 15/10/2026, D-28) : les calculs `{% set %}` les plus utiles d'un site importé deviennent des réglages simples — titre de l'onglet (format « titre | nom du site »), adresse officielle et image de partage (paramètres de la page et du site), menu du site (liste de liens), contenus mis en avant (choix dans une liste) ; le panneau Variables reste fermé par défaut | `revel-render`, `revel-edit`, `revel-ui` |

## Choix retenus (15/10/2026, voir DECISIONS.md)

- **D-23 — langues** : un dossier de contenus et de données par langue, adresses préfixées (`/en/…`), comme
  Webflow Localization.
- **D-24 — polices** : fichiers copiés dans le site (aucune requête vers Google).
- **D-25 — synchronisation** : enregistrement et envoi sur le dépôt GitHub du site à la publication, ou à la
  demande.
- **D-26 — aperçu et export** : aperçu sans génération préalable ; « Générer » devient « Exporter ».
- **D-27 — schéma des collections** : `.pages.yml` de Pages CMS, lu et modifié par Revel (pas de copie).

## Avancement

| N | État |
|---|---|
| 3.1 | fait : commandes `SetEntryField`, `SetEntryBody`, `CreateEntry`, `WriteEntry` appliquées par le cœur (`revel-project::entries` : champ simple réécrit sur place avec son style, champ composé réécrit en bloc à sa place, champ ajouté en fin d'en-tête, dates ISO sans guillemets, relecture avant écriture ; inverse = texte exact du fichier) ; vue CMS : fiche modifiable (texte, texte long, date, oui/non, nombre, liste en pastilles, image, objet en JSON), texte du contenu, nouveau contenu (nom de fichier tiré du titre), suppression confirmée, annuler/rétablir (boutons, Ctrl+Z) dans l'historique du site ; l'éditeur relit pages et canevas. Essais : 4 essais d'écriture YAML, 1 essai du cœur (tout annulé octet pour octet, chemins refusés) ; essais de l'interface 97/97 |
| 3.2 | fait : éditeur de texte riche (`richtext.rs`) : paragraphes, titres 2 à 4, citation, gras, italique, liens (Ctrl+K), listes à puces et numérotées, retrait de la mise en forme, collage en texte brut, source Markdown ou HTML ; écrit en Markdown (texte du contenu) ou en HTML nettoyé (champs importés de WordPress : liste blanche de balises, sans styles), réécrit seulement s'il a changé ; texte sur plusieurs lignes gardé en bloc littéral dans l'en-tête. Essais de l'interface : 98/98 |
| 3.3 | fait. **Lecture** : `revel-core::pagescms` lit `.pages.yml` (composants remplacés, `rich-text`, blocs, listes, motifs ; tri par défaut `view.default` comme Pages CMS, écriture ancienne `view.sort` acceptée) ; vue CMS : nom des collections, champ principal, colonnes et tri du schéma, libellés au-dessus des champs, aide au survol (ⓘ), champs obligatoires (*) et motifs vérifiés avant écriture (valeur refusée signalée, rien d'écrit), date avec heure écrite au format du schéma, choix (liste ou pastilles), références (titres affichés, ajout depuis une liste, maximum respecté), objet en sous-fiche, blocs en JSON, champs masqués et valeurs hors schéma repliés (« Autres valeurs » ; avertissement quand Pages CMS les retirerait, bouton « Ajouter au schéma ») ; nouveau contenu : nom de fichier du modèle (`{fields.slug}.md`), adresse tirée du titre, valeurs par défaut, date du jour ; collections d'Eleventy sur un même dossier regroupées. **Modification** (mode « Champs ») : liste des champs (type, nom dans les fichiers, marques titre, colonne, masqué), ordre au glisser ou aux flèches, réglages du champ (libellé, aide, type avec confirmation si des valeurs existent, obligatoire, plusieurs valeurs, masqué, colonne du tableau, titre des contenus, heure d'une date, format d'un texte riche, choix, collection et libellé d'une référence, motif), nouveau champ par type, champ retiré (valeurs gardées dans les contenus) ; nouvelle collection (site et `.pages.yml`, champs titre et adresse). Commandes `SetCmsField`, `AddCmsField`, `RemoveCmsField`, `MoveCmsField`, `SetCmsCollection`, `CreateCmsCollection`, inverses `WriteCmsConfig` et `RemoveCmsCollection` (texte exact) ; `revel-project::pagesyml` réécrit seulement la clé, le champ ou la liste visés, commentaires gardés, chaque écriture relue. Essais : 5 essais de `pagesyml` (dont un aller-retour sur un vrai `.pages.yml`, lancé à la demande), 4 essais du cœur, essais de l'interface 119/119 (`.pages.yml`, document et contenus identiques octet pour octet après annulation). Une nouvelle collection n'a pas encore de page par contenu (gabarit de collection : 3.4) ; groupes de champs et blocs : modification prévue (3.7) |
| 3.4 | fait (D-32) : commandes `AddPage`, `RemovePage`, `SetPagePath` (adresse valide et libre), `SetPageDraft`, `SetPageName`, `SetRedirects`, `SetSeoSettings`, `SetDataValue` (clé d'un fichier de données écrite sur place ou ajoutée, `revel-project::yamlpath`, inverse au texte exact) ; panneau Pages : nouvelle page (gabarit du site le plus utilisé, ouverte aussitôt), dossiers d'adresse (créer, renommer avec redirections, supprimer vide), menu contextuel (paramètres, dupliquer, brouillon, déplacer dans un dossier, copier l'adresse, supprimer avec confirmation), brouillons marqués ; paramètres de la page par-dessus la colonne (nom, adresse avec redirection 301, brouillon, titre, description, image de partage, visibilité par les moteurs, aperçu du résultat de recherche, source des valeurs) ; génération : brouillons écartés, balise `noindex`, plan du site et `robots.txt` produits si le site n'en a pas ; paramètres du site : rubrique Référencement (plan du site, robots.txt) et redirections modifiables ; nouveau site : site de départ (gabarit de base avec référencement, accueil, feuille de style, dossier des médias). Essais : 3 essais du cœur, 2 du moteur, 1 de `yamlpath`, 2 de `revel-edit` ; essais de l'interface 152/152 |
