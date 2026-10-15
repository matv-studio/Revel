# Étape 2 — Éditeur visuel

Objectif : modifier un site dans Revel comme dans Webflow ou Framer — sélectionner un élément sur la page
ou dans les calques, changer ses styles par point de rupture, ses réglages et ses textes, annuler et rétablir —
sans écrire de code, et sans perdre l'équivalence de la sortie avec le site d'origine.

**Critères d'acceptation**
1. Le site cobaye s'ouvre dans l'éditeur ; chaque page s'affiche dans le canevas comme le site généré.
2. Tout élément d'une page, d'un gabarit ou d'un composant se sélectionne sur la page ou dans les calques.
3. Un style modifié sur un point de rupture s'applique en direct ; annuler rend `site.revel.json` identique
   octet pour octet à l'état précédent.
4. Les styles du cobaye sont importés dans le modèle de Revel (D-18) ; le CSS généré est équivalent à la feuille
   d'origine (comparaison en CI) et les pages rendues sont identiques à l'image près aux trois largeurs.
5. Un texte cliqué sur la page se modifie sur place, qu'il soit écrit dans la page ou dans un fichier de données
   (D-19) ; dans ce cas seul ce texte change dans le fichier, qui reste lisible par Pages CMS.
6. Les modifications passent toutes par des **commandes** (D-13), utilisables plus tard par le serveur MCP.

## Choix retenus (14/10/2026, voir DECISIONS.md)

- **D-18 — styles détenus par Revel** (modèle Webflow) : la feuille de style d'un site importé devient des styles
  du modèle (sélecteur, état, point de rupture, déclarations, dans l'ordre d'origine) ; Revel génère le CSS.
- **D-19 — textes liés** : un texte issu d'un fichier de données ou d'un contenu (YAML, front matter) se modifie en
  cliquant dessus ; Revel écrit ce fichier sans toucher au reste. Le CMS édite les mêmes fichiers ; Revel relit les
  fichiers modifiés hors de l'application. La synchronisation avec GitHub (là où Pages CMS enregistre) relève de
  l'étape 3.

## Avancement

| N | État au 14/10/2026 |
|---|---|
| 2.0 | fait : schéma 2 (identifiants stables, déterministes à l'import), `revel-edit` (commandes réversibles, suites atomiques, intentions développées, historique avec fusion des saisies), sessions d'édition dans `revel-core` avec enregistrement immédiat, commandes Tauri, `revel apply` ; 40 suites aléatoires de 30 commandes annulées puis rétablies à l'octet près ; cobaye réimporté (`af1ac84`), toujours équivalent à Eleventy |
| 2.1 | fait : schéma 3 (feuille de style du modèle : règles avec identifiant, requêtes média, commentaires, blocs bruts ; points de rupture), `revel-style` (lecture sans réécriture des valeurs, génération, forme canonique, points de rupture, place d'une nouvelle règle dans la cascade comme Webflow), import de la feuille du cobaye (395 règles ; 900, 860 et 600 px), CSS publié généré par Revel, commandes de style réversibles dans `revel-edit` (valeurs vérifiées). Équivalence : CSS identique sous forme canonique ; styles calculés identiques sur les 66 pages du cobaye à 1280, 900, 860, 600 et 375 px, avec et sans « mouvement réduit » (750 570 éléments et pseudo-éléments comparés) et captures identiques au bruit de dessin près (330 paires, écart max 35/255) ; en CI, 11 pages représentatives à chaque largeur, styles calculés et captures (bruit de dessin ≤ 30/255). Cobaye `0dd004c` |
| 2.2 | fait : rendu pour l'éditeur (repères `data-rv` et `<!--rv:N-->`, adresses des textes, origine des valeurs affichées à travers `set`, boucles et filtres), scripts du site neutralisés, protocole `revel-canvas` (page, CSS du document, images, copies), script du canevas (survol, sélection, messages, rechargement de la feuille de style), `revel canvas`, `canvas-check`, `canvas-serve`. Cobaye : 66 pages identiques à la génération une fois les repères retirés ; canevas servi = site généré (styles calculés et captures, 0 écart) ; essais de bout en bout 9/9 sur 3 pages, scripts du site actifs ou non |
| 2.3 | fait : onglet « Éditeur » (plein écran) : choix de la page (pages fixes et contenus par collection), points de rupture du site (largeur du cadre), zoom ajusté ou fixe, scripts du site activables, sélection et survol transmis par le canevas (élément, propriétaire dans la barre d'état), annuler/rétablir (boutons, Ctrl+Z, Ctrl+Maj+Z, Ctrl+Y, depuis l'interface ou le canevas) ; copie locale du document tenue à jour par les commandes appliquées ; style modifié : feuille rechargée sans recharger la page, autre modification : page rechargée au même défilement. Essais de bout en bout de l'interface dans Chromium (API Tauri simulée reliée à un vrai cœur par `canvas-serve --bridge`) : 11/11 |
| 2.4 | fait : panneau « Calques » : arbre de la page affichée (gabarits emboîtés, contenu de la page à l'emplacement du gabarit, composants utilisés, conditions, boucles, variables, textes), `head` et composants repliés par défaut ; sélection synchronisée dans les deux sens (ligne rendue visible) ; survol ; monter, descendre, envelopper, dupliquer (Ctrl+D), supprimer (Suppr) ; glisser-déposer avant, après ou dans un élément, à l'intérieur d'une même page, d'un même gabarit ou d'un même composant ; élément ajouté sélectionné. Essais de bout en bout : 18/18 |
| 2.5 | fait : panneau « Réglages » de l'élément sélectionné : balise (liste des balises courantes), identifiant, classes (ajout, retrait ; texte brut si elles sont calculées), lien (adresse, nouvel onglet avec `rel="noopener"`), image (source et texte alternatif, littéraux ou expressions), condition, boucle, autres attributs (modifier, retirer, ajouter `nom=valeur`) ; panneau « Ajouter » (section, conteneur, titre, paragraphe, lien, bouton, image, liste) dans l'élément sélectionné ou juste après, textes par défaut traduits ; un dessin SVG se sélectionne en entier ; cadre « ordinateur » au moins aussi large que le plus grand point de rupture. Essais de bout en bout : 26/26 |
| 2.6 | fait : panneau « Styles » (onglet par défaut de l'inspecteur) : sélecteur choisi parmi les classes de l'élément ou leur combinaison, état (survol, focus clavier, appui), ajout d'une classe ; sections Espacement (marges extérieures et intérieures côté par côté), Mise en page, Taille, Position, Typographie, Arrière-plans, Bordures, Effets, et toutes les déclarations de la règle (raccourcis, propriétés personnalisées, `!important`) ; valeur de la règle mise en évidence, valeur effective de l'élément (calculée dans le canevas) en indication ; première valeur à un point de rupture : règle créée à sa place dans la cascade ; mise à jour en direct sans recharger la page |
| 2.7 | fait : double-clic sur un texte du canevas : modifiable sur place (Entrée ou clic ailleurs valide, Échap annule). Texte écrit dans la page, un gabarit ou un composant : `SetText` ; texte lié (D-19) : `SetBound`, écrit par le cœur dans sa source — fichier de données YAML ou en-tête d'un contenu (écriture ciblée : seule la valeur change, style de guillemets gardé, fichier relu avant écriture), ou variable de page ou de gabarit ; annulable comme les autres commandes, fichier rendu identique octet pour octet. Typographie française de la page non recopiée dans la source (espaces insécables, apostrophe) si la source ne l'utilisait pas. Fichiers modifiés hors de Revel (Pages CMS, éditeur de texte) : détectés (empreinte toutes les 2 s), document et page relus. Essais de bout en bout : 42/42 |
| 2.8 | fait : points de rupture du site modifiables (ajout « jusqu'à » ou « à partir de » une largeur, retrait d'un point sans styles), commande `SetBreakpoints` vérifiée (base en premier, largeurs uniques) ; styles réglés au point de rupture affiché, cascade du plus large au plus étroit. Essais de bout en bout : 34/34 |
| 2.10 | fait : panneau de styles refait pour un public non technique (D-20, demande du 14/10/2026) après étude des panneaux de Webflow, Framer et Figma (`docs/DESIGN.md`) : noms simples (nom CSS en info-bulle), origine de chaque valeur (réglée ici, héritée d'un écran plus large ou de l'état de base) et réinitialisation, champs numériques à glisser avec calculs et menu d'unités expliqué, boîte des marges, disposition en pile ou grille avec boîte d'alignement 3 × 3, tailles, position avec préréglages, texte (police du site, graisses nommées, alignement et casse à icônes), fond, bordure par côté et angles indépendants, opacité, ombres, curseur ; sélecteur de couleur (plan saturation-luminosité, teinte, opacité, pipette, Hex / RVB / TSL / TSV, couleurs et variables du site, contraste) ; aperçu en direct pendant un geste, une seule étape d'annulation ; raccourcis CSS du site (`margin: 0 auto`, `border`, `background`) lus et modifiés sur place (`revel-style::values`, testé). Essais de l'interface : 59/59 (aucun libellé tronqué en français ni en anglais) |
| 2.11 | fait : retours du 14/10/2026 (D-21) : espace de travail modulable (panneaux déplaçables entre colonnes, groupes et fenêtres flottantes, redimensionnables, fermés et rouverts, Ctrl+\, disposition gardée), onglets en texte, panneaux Pages, Composants, Médias, CMS (lecture ; édition à l'étape 3) et Interactions (étape 6) ; calques avec repères de niveau, recherche, clavier, menu contextuel et actions en bas ; panneau de styles au code couleur de Webflow, menus d'unités courts, boîte des marges symétrique, aides en info-bulles, glissière retravaillée ; fil d'Ariane de la sélection, zoom Ctrl+0 / Ctrl+1 / Ctrl+molette ; accueil en grille de sites avec recherche. Essais de l'interface : 74/74 |
| 2.12 | fait : retours du 15/10/2026 (D-22) : barre supérieure à onglets de vue raccordés au plateau (Éditeur, CMS, Localisation, Paramètres ; Sites, Réglages à l'accueil), menu du site, accueil avec tri et vignettes teintées, création d'un site sans fenêtre du système (dossier proposé dans Documents/Revel), paramètres du site en rubriques (Général, Publication, Génération, Redirections ; Référencement, Polices, Code personnalisé, Sauvegardes à l'étape 3, Formulaires à l'étape 5), vue CMS (collections, tableau triable, recherche, fiche, ouverture de la page dans l'éditeur), vue Localisation (étape 3), éditeur gardé en place entre les vues ; panneau « Ajouter » en familles, aides en info-bulles, éléments prévus visibles ; canevas : déplacement de la vue (espace, bouton du milieu, molette autour du cadre), zoom autour du pointeur et sur la sélection (Maj+0/1/2), glisser-déposer depuis « Ajouter » et dans la page (Alt : copie) avec emplacement montré, Entrée / Maj+Entrée / Tab, Alt+flèches, Ctrl+C/X/V, Ctrl+G / Ctrl+Maj+G, menu contextuel. Essais de l’interface : 91/91 ; essais du canevas 9/9 sur 3 pages |
| 2.13 | fait : retours du 15/10/2026 (D-26) : aperçu sans génération préalable (protocole `revel-preview`, pages rendues à la demande depuis le document en cours, sans repères, scripts du site actifs, à la page affichée) ; « Génération » devient « Export » (copie des fichiers du site dans un nouveau dossier choisi, « Afficher le dossier ») |
| 2.9 | fait : raccourcis réunis dans une seule table, valables dans l'interface, les calques et le canevas (Ctrl+Z, Ctrl+Maj+Z, Ctrl+Y, Ctrl+D, Suppr, Échap) et rappelés dans un aide-mémoire (bouton « ? ») ; état d'enregistrement dans la barre d'état (« Enregistrement… », « Enregistré à HH:MM ») ; essais de l'application elle-même pilotée par WebDriver (`tools/app-e2e.mjs`, job CI « Application de bureau » sous Linux dans Xvfb et sous Windows) : protocole du canevas, feuille de style, sélection, calques, suppression et annulation au clavier dans le canevas, fichiers du site intacts ; essais de l'interface dans Chromium : 48/48 ; version 0.2.0 et installateurs de test (workflow *Installateurs*) ; étiquette `step-2` |

## Sous-étapes

Chaque sous-étape = commits + étiquette `step-2.N`, posée par le workflow `tag.yml` sur le dernier commit de `main`
dès la sous-étape terminée : GitHub refuse au jeton des workflows une étiquette sur un commit plus ancien dont les
fichiers `.github/workflows/` diffèrent de `main` (droit `workflows` réservé). C'est pourquoi `step-2.0` n'existe
pas : la sous-étape 2.0 est incluse dans `step-2.1`, la 2.2 dans `step-2.3`, la 2.4 dans `step-2.5`, les 2.6 et 2.8
dans `step-2.7`.

| N | Contenu | Modules |
|---|---|---|
| 2.0 | **Commandes et historique** : identifiants stables des nœuds (schéma v2, migration), commandes sérialisables et réversibles (texte, attributs, insertion, déplacement, suppression, duplication, styles), historique annuler/rétablir, enregistrement automatique | nouveau `revel-edit`, `revel-model`, `revel-core` |
| 2.1 | **Styles dans le modèle** : feuille de style ordonnée (règles, médias, états, blocs bruts `@font-face`, `@keyframes`) ; import d'un CSS existant (lecteur propre, valeurs conservées telles qu'écrites) ; points de rupture déduits des `@media` ; génération du CSS ; commandes de style réversibles ; équivalence vérifiée en CI (CSS comparé après normalisation, styles calculés de chaque élément comparés dans Chromium à plusieurs largeurs) | nouveau `revel-style`, `revel-edit`, `revel-project`, `revel-import` |
| 2.2 | **Rendu pour l'éditeur** : une page rendue à la demande avec repères des nœuds (`data-rv`), liaisons des textes vers leur source, script de l'éditeur ; servie par un protocole propre à l'application (sans port réseau) ; scripts du site désactivés par défaut | `revel-render`, `revel-core`, `revel-app` |
| 2.3 | **Canevas** : page dans un cadre, survol et sélection, étiquette de l'élément, largeur par point de rupture, zoom ; mise à jour en direct des styles sans recharger | `revel-ui` |
| 2.4 | **Calques** : arbre de la page, du gabarit et des composants ; blocs logiques visibles (condition, boucle, composant) ; glisser-déposer, dupliquer, supprimer, envelopper | `revel-ui`, `revel-edit` |
| 2.5 | **Réglages et insertion** : balise, attributs, lien, image (source, texte alternatif), classes ; panneau « Ajouter » (section, conteneur, titre, paragraphe, lien, bouton, image, liste) | `revel-ui`, `revel-edit` |
| 2.6 | **Panneau de styles** : sélecteur (classes, classes combinées, états), sections Mise en page, Espacement, Taille, Position, Typographie, Arrière-plans, Bordures, Effets, Propriétés personnalisées ; valeur héritée ou modifiée selon le point de rupture | `revel-ui`, `revel-style` |
| 2.7 | **Textes** : édition sur place des textes de la page et des textes liés (écriture ciblée du YAML et du front matter) ; relecture des fichiers modifiés hors de Revel | `revel-edit`, `revel-project`, `revel-ui` |
| 2.8 | **Points de rupture** : liste du site (ajout, largeurs), cascade du plus large vers le plus étroit (comme Webflow) | `revel-style`, `revel-ui` |
| 2.9 | Raccourcis clavier, état d'enregistrement, essais de bout en bout (Xvfb), installateurs, étiquette `step-2` | CI |
| 2.10 | Panneau de styles au niveau de Webflow, Framer et Figma (D-20) : noms simples, contrôles visuels, sélecteur de couleur | `revel-ui`, `revel-style` |
| 2.11 | Espace de travail modulable, panneaux prévus, calques et accueil retravaillés (D-21) | `revel-ui` |

## Principes techniques

- **Identifiants** : chaque nœud structurel (élément, image, condition, boucle, composant) reçoit un identifiant
  stable. À l'import il est déterministe (même import, mêmes identifiants) ; un nœud créé dans l'éditeur reçoit un
  identifiant aléatoire. Le texte et les expressions sont adressés par leur parent.
- **Commandes** : `revel-edit` ne dépend que de `revel-model`. Une commande s'applique au document et renvoie
  la commande inverse ; l'historique garde les paires. Le même code sert à l'interface, à la ligne de commande et au
  futur serveur MCP.
- **Ordre du CSS** : la cascade dépend de l'ordre des règles. Les règles importées gardent leur ordre ; une règle
  créée dans l'éditeur pour un point de rupture est ajoutée après les règles existantes (elle l'emporte, comme dans
  Webflow). Le panneau regroupe les règles par sélecteur pour l'affichage.
- **Correspondance élément ↔ styles** : le canevas demande au navigateur quelles règles s'appliquent à l'élément
  (`Element.matches`), comme les outils de développement ; le panneau édite la règle de la classe principale et
  liste les autres.
- **Canevas** : page servie par un protocole personnalisé de Tauri (`revel-canvas://localhost/…` sous macOS et
  Linux, `http://revel-canvas.localhost/…` sous Windows) ; échanges entre le canevas et l'interface par `postMessage`.
- **Textes liés** : en mode éditeur, une expression simple (`pages.agence.intro`, `item.data.title`) est rattachée à
  sa source (fichier de données, entrée de collection, variable de page). L'écriture remplace seulement la valeur
  dans le fichier (repérage par positions dans le YAML), le reste du fichier reste octet pour octet identique.
- Textes riches Markdown (corps des articles) : édition à l'étape 3 avec le CMS.

## Bibliothèques (versions crates.io du 14/10/2026)

| Besoin | Choix | Raison |
|---|---|---|
| Lecture et écriture du CSS | `lightningcss` 1.0.0-alpha.72 | déjà retenu à l'étape 0 ; analyse complète (sélecteurs, médias, `@keyframes`) |
| YAML avec positions | à vérifier au moment de 2.7 (`saphyr-parser` envisagé) | écriture ciblée sans réécrire le fichier |
| Surveillance des fichiers | à vérifier au moment de 2.7 (`notify` envisagé) | relecture des fichiers modifiés par le CMS ou Git |

## Sources

- Points de rupture et cascade de Webflow : https://help.webflow.com/hc/en-us/articles/33961300305811
- Panneau de styles de Webflow : https://help.webflow.com/hc/en-us/articles/33961362040723-Style-panel-overview
- Protocoles personnalisés de Tauri : https://docs.rs/tauri/latest/tauri/struct.Builder.html
