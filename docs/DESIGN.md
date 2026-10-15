# Système de design de Revel

L'interface fait partie du produit au même titre que le moteur. Ce document est la référence :
tout écran nouveau est construit avec ces jetons et ces composants ; si un besoin n'est pas couvert,
on étend d'abord le système (ici et dans `apps/revel-ui/styles/`), puis on l'utilise.

Références d'usage : Figma (densité, panneaux, contrôles compacts), Framer et Webflow
(structure d'un outil de création de sites : barre supérieure, panneau gauche, canevas, inspecteur).
Revel n'en reprend ni les couleurs ni les marques.

## Principes

1. **Le contenu de l'utilisateur est la seule couleur.** L'interface est en gris neutres.
   Aucune couleur décorative ; les couleurs fonctionnelles restent discrètes : erreur, réussite, et dans le
   panneau de styles l'origine des valeurs (bleu réglé ici, orange hérité, comme Webflow — D-21).
2. **Un seul accent, choisi par l'utilisateur.** *Réglages > Couleur d'accent* : « Aucune » par défaut.
   Sans accent, l'action principale est en contraste inversé (blanc sur sombre, noir sur clair).
   L'accent ne sert qu'à trois choses : action principale, sélection, focus.
3. **Une action principale par zone.** Une seule `.btn--primary` visible par section ou barre.
4. **Densité d'outil.** Texte 13 px, contrôles 28 px, grille de 4 px : beaucoup d'information sans
   surcharge, comme Figma. Les pages de réglages restent aérées.
5. **Peu de texte, mots simples, verbes d'action.** Les explications vont en info-bulle (D-21). Un bouton dit ce qu'il fait (« Générer le site », « Publier ») ;
   le message de fin reprend le même verbe (« Publication terminée »). Phrases en casse normale,
   jamais de libellés en capitales. Une erreur dit ce qui s'est passé et comment y remédier.
6. **Rien ne bouge sans raison.** Seuls les retours à une action sont animés (progression, bascule) ;
   les préférences « réduire les animations » du système sont respectées.
7. **Accessible par défaut.** Focus visible au clavier, contrastes AA, libellés sur les boutons-icônes,
   rôles ARIA sur les choix exclusifs.

## Jetons (`styles/tokens.css`)

| Famille | Jetons | Usage |
|---|---|---|
| Gris | `--gray-0` … `--gray-900` | Valeurs brutes, jamais utilisées directement par un composant |
| Surfaces | `--bg`, `--surface`, `--surface-raised`, `--surface-hover`, `--surface-active` | Fond (canevas), panneaux, blocs, survol, élément actif |
| Cadre | `--chrome`, `--stage-line` | Cadre de la fenêtre et barre supérieure autour du plateau ; contour du plateau et de l'onglet actif |
| Traits | `--line`, `--line-strong` | Séparateurs, bordures de contrôles |
| Texte | `--text`, `--text-muted`, `--text-faint` | Principal, secondaire, indications |
| Action | `--primary`, `--primary-ink`, `--focus`, `--selection` | Dérivés de l'accent (ou du contraste neutre) |
| États | `--danger-text`, `--danger-bg`, `--success-text`, `--danger-solid` | Erreur, réussite, bouton de confirmation destructrice |
| Superposition | `--overlay`, `--shadow-dialog` | Fond et ombre des boîtes de dialogue |
| Texte (tailles) | `--text-xs` 11, `--text-sm` 12, `--text-md` 13, `--text-lg` 15, `--text-xl` 20 | Libellés, aide, interface, titre de section, titre de page |
| Graisses | `--weight-regular` 420, `--weight-medium` 530, `--weight-strong` 620 | Police variable Geist |
| Espaces | `--s-1` 4 … `--s-10` 40 | Grille de 4 px |
| Contrôles | `--control-sm` 24, `--control-md` 28, `--control-lg` 32 | Hauteurs |
| Rayons | `--radius-sm` 4, `--radius-md` 6, `--radius-lg` 8 | Puces, contrôles, sections |
| Structure | `--topbar-h` 44 | Barre supérieure (onglets compris) |
| Mouvement | `--ease`, `--fast` 120 ms, `--normal` 200 ms | Transitions de retour d'action |

Thèmes : sombre (défaut), clair, ou système (`data-theme` absent sur `<html>`).
L'accent est appliqué par l'application en définissant `--accent` et `--accent-ink` sur `:root`
(`--accent-ink` est calculé : noir ou blanc selon la luminance de l'accent).

**Police** : Geist (variable, licence OFL, fichiers dans `apps/revel-ui/fonts/`), intégrée pour un
rendu identique sous Windows, macOS et Linux. Police à chasse fixe du système (`--font-mono`) uniquement pour
les valeurs techniques à comparer ou recopier (empreinte d'un serveur).

## Structure (`styles/shell.css`)

```
  logo  Alunites ▾          ╭ Éditeur ╮  CMS   Localisation   Paramètres          Aperçu  Publier
╭──────────────────────────╯         ╰──────────────────────────────────────────────────────────╮
│ plateau : la vue choisie (éditeur, CMS, localisation, paramètres du site)                      │
│ accueil : onglets « Sites » et « Réglages » (réglages de Revel)                                 │
╰────────────────────────────────────────────────────────────────────────────────────────────────╯
```

- **Plateau** (`.main`) : un seul espace de travail, à coins arrondis, entouré d'un contour fin et un peu en
  retrait du bord de la fenêtre. Le cadre de la fenêtre (`--chrome`) est d'une autre teinte que le plateau.
- **Onglets de vue** (`.view-tab`) : en texte, en bas de la barre supérieure. L'onglet actif prend la teinte du
  haut du plateau (`--tab-bg` : barre d'outils pour l'éditeur et le CMS, fond de page ailleurs) et s'y raccorde
  par des coins incurvés, comme un intercalaire : onglet et vue se lisent comme un seul espace (demande du
  15/10/2026). Pas de fil d'Ariane dans la barre supérieure.
- **Site ouvert** : son nom à gauche, avec un menu (autres sites récents, tous les sites, réglages de Revel) ;
  « Aperçu » et « Publier » à droite. Un clic sur un site de l'accueil l'ouvre dans l'éditeur ; ses paramètres
  restent à un clic (bouton de la carte).
- **Changer de vue ne recharge rien** : l'éditeur reste monté, caché, pendant que les autres vues sont affichées.
- **Activité** (génération, publication) : ligne de 2 px en haut de la fenêtre, visible depuis n'importe quelle vue.
- **Paramètres du site** : rubriques à gauche (comme les réglages d'un site Webflow) — Général, Publication,
  Export, Référencement, Redirections, Polices, Code personnalisé, Formulaires, Sauvegardes ; les rubriques
  prévues sont affichées en gris avec leur étape.
- **Aperçu** (D-26) : le site tel qu'il sera publié, dans une fenêtre, à la page affichée dans l'éditeur ; rien à
  générer avant. **Export** : fichiers du site copiés dans un dossier choisi, « Afficher le dossier » ensuite.

## Composants (`styles/components.css`)

| Composant | Classe | Règles |
|---|---|---|
| Page | `.page`, `.page--narrow`, `.page-head` | Titre `h1` à gauche, actions de la page à droite |
| Section | `.section`, `.section-head` | Bloc de contenu d'un même sujet ; titre `h2` |
| Bouton | `.btn`, `.btn--primary`, `.btn--danger` | Icône 16 px facultative à gauche du libellé ; une seule action principale par zone ; `--danger` uniquement dans une confirmation |
| Bouton-icône | `.icon-btn` | Actions répétées sur des lignes ; `aria-label` obligatoire |
| Barre d'outils | `.toolbar` | Groupe de boutons, espacement 8 px |
| Champ | `.field` + `.label` + `.input` (+ `.help`) | Libellé au-dessus, 11 px ; aide sous le champ (exemple, où trouver la valeur) |
| Liste déroulante | `.select` > `select.input` | Chevron dessiné par Revel, identique sur les trois systèmes |
| Formulaire | `.form-grid` (libellé / contrôle), `.form-cols` (2 colonnes de champs) | Réglages ; saisie d'un hébergement |
| Choix exclusif | `.segmented` (`role="radiogroup"`, boutons `role="radio"`) | 2 à 4 options courtes |
| Interrupteur | `.switch` | Option activable immédiatement |
| Couleurs | `.swatches`, `.swatch` | Réglage de l'accent |
| Sites | `.site-grid` > `.site-card` (vignette à monogramme teintée d'après le nom, nom, date ; paramètres et retrait au survol) ou `.table` | Accueil : grille ou liste, recherche, tri (récents, nom) ; clic : éditeur |
| Nouveau site | `.dialog--form` + `.path-field` | Nom, langue, emplacement proposé (`Documents/Revel/nom-du-site`) ; la fenêtre du système ne s'ouvre que sur « Changer… » |
| Onglets de vue | `.views` > `.view-tab` | Voir *Structure* |
| Menu | `.menu` > `.menu-item` (`.menu-title`, `.menu-sep`) | Menu du site, menu contextuel du canevas |
| Rubriques | `.settings-nav` > `.settings-nav-item` (`.is-planned` + numéro d'étape) | Paramètres du site |
| Vue CMS | `.cms` : `.cms-cols` (collections), `.cms-main` (tableau triable, recherche, annuler/rétablir, nouveau contenu), `.cms-item` (fiche du contenu) | Fiche : un contrôle par type de champ (schéma de `.pages.yml`, sinon déduit de la valeur), libellé au-dessus (case à cocher sur sa ligne), aide au survol (ⓘ), champ obligatoire marqué `*` ; valeur écrite à la sortie du champ, refusée si obligatoire et vide ou hors motif (bordure et message rouges, rien d'écrit) ; choix multiples en pastilles à cocher, références en pastilles + liste « Ajouter… » ; objet dans un cadre légendé ; champs cachés et valeurs hors schéma repliés en bas (« Autres valeurs ») ; la fiche n'est pas redessinée pendant la saisie (le focus reste) ; suppression confirmée, annulable. Fiche ouverte : dossier de la collection masqué (au survol du nom). Mode « Champs » (bouton bascule bleu quand actif) : liste des champs en lignes (poignée, type, libellé, nom technique en chasse fixe, marques), ligne choisie entourée de bleu, trait bleu à l'emplacement pendant le glisser ; réglages du champ à droite (même gabarit que la fiche, interrupteurs groupés) ; « Nouveau champ » : menu des types (groupes de champs et blocs désactivés, prévus) ; « + » de la colonne des collections : nouvelle collection |
| Liste de textes | `.tags-input` > `.chip--tag` + `.tags-add` | Pastilles retirées par ×, ajout par Entrée |
| Texte riche | `.rt` : `.rt-bar` (type de paragraphe, gras, italique, lien, listes, retirer la mise en forme · source), `.rt-body` (modifiable), `.rt-code` (source) | Boutons sans prise de focus (la sélection reste) ; collage en texte brut ; jamais dans un `<label>` (le clic irait au premier bouton) |
| Tableau | `.table`, `.row-link`, `.table--compact` | Listes de sites, de collections ; ligne entière cliquable |
| Faits | `.facts`, `.facts--inline` | Chiffres clés ; paires libellé / valeur |
| Puce | `.chip` | Étiquette courte (environnement) |
| Liste | `.list-plain` (`.is-blocking`) | Avertissements, points à vérifier |
| Retour | `.notice--error`, `.status--ok`, `.empty` | Erreur (avec action de correction si elle existe), réussite, état vide (avec l'action qui le remplit) |
| Valeur technique | `.code`, `.code--large` | Empreinte, clé : chasse fixe, sélection en un clic ; `--large` pour un code à recopier (autorisation GitHub) |
| Compte relié | `.account`, `account.rs` | État de connexion et action (connecter, déconnecter) ; le même composant dans Réglages et dans un formulaire |
| Sous-bloc | `.subsection` | Groupe distinct à l'intérieur d'une section (compte requis par un formulaire) |
| Boîte de dialogue | `.overlay` > `.dialog` (`dialog.rs`) | Confirmation ou saisie ; remplace `confirm()`/`prompt()` du système ; Échap ferme ; action destructrice : focus sur « Annuler » |
| Panneau de styles | `styles/inspector.css`, `src/styles/` | Voir la section *Panneau de styles* |
| Libellé de propriété | `.style-label` (`.is-here`, `.is-inherited`) + `.dot` | Pastille dans la marge (texte aligné) ; clic : origine et *Réinitialiser* ; Alt+clic : réinitialise ; glisser : change le nombre |
| Champ numérique | `.num` (`NumBox`) | Calculs, flèches (Maj ×10, Alt ×0,1), menu d'unités expliqué (`.num-unit`), préfixe à glisser |
| Choix à boutons | `.seg` (`.is-on` réglé ici, `.is-implied` en vigueur sans être réglé) | Texte ou icônes ; 2 à 5 options |
| Bascule | `.tool-toggle` | Italique, souligné, angles indépendants |
| Boîte d'alignement | `.align-box` | 3 × 3 ; positions d'une pile ou d'une image de fond |
| Boîte des marges | `.bm-box` | Marge extérieure autour de la marge intérieure ; décalages d'une position |
| Couleur | `.color-input` + `.cp` | Pastille, valeur, sélecteur (plan, teinte, opacité, pipette, modèles, couleurs du site, contraste) |
| Menu flottant | `.pop` (`Pop`) | Ouvert par un `.pop-trigger` ; fermé par un clic ailleurs ou Échap (avant tout raccourci de l'éditeur) |

Icônes : `apps/revel-ui/src/icons.rs`, 16 px, trait de 1,5 px, extrémités arrondies, dessinées pour Revel.

## Espace de travail de l'éditeur (D-21)

```
┌ barre d'outils : [page ▾]¹ [modèle ou design propre]² · points de rupture · zoom · affichage · scripts ··· panneaux ? annuler rétablir ┐
├ colonne gauche ──────────┬ canevas (fixe) ────────────────────────────┬ colonne droite ───────────┤
│ Pages Composants Médias   │                                            │ Styles Réglages Interactions│
│ ───────────────────────── │        page à la largeur du point          │                            │
│ Calques Variables Ajouter │        de rupture, zoom                    │                            │
│   (recherche, arbre,      │                                            │                            │
│    actions en bas)        │   [fenêtre flottante possible]             │                            │
└ fil d'Ariane de l'élément sélectionné · propriétaire ········· enregistrement · largeur ──────────┘
¹ seulement quand le panneau Pages n'est pas visible ; ² seulement pour un contenu de collection
```

| Élément | Règles |
|---|---|
| Onglet (`.dock-tab`) | Texte seul ; actif en relief (fond, contour) ; glisser pour déplacer, clic pour afficher ; clic droit : détacher (ou ancrer), fermer |
| Fenêtre flottante | Déplacée par tout son bandeau (onglet seul compris) ; Maj pendant le déplacement : zone d'ancrage montrée, la fenêtre s'y range au relâchement |
| Menu contextuel (`.ctx-menu`) | Un par zone (onglet, calque, page, média, composant, dossier, variable, canevas) ; une ligne par action : icône, libellé, raccourci à droite (jamais répété dans le libellé) ; fermé par un clic ailleurs, Échap. Menu du navigateur désactivé hors des champs de saisie |
| Dépôt | Bande d'onglets : rejoint le groupe ; haut ou bas d'un panneau : nouveau groupe au-dessus ou au-dessous ; colonne vide : nouveau groupe ; canevas : fenêtre flottante. Zone de dépôt en bleu |
| Groupe | Menu ⋯ : liste des onglets, détacher, fermer ; poignée de hauteur entre deux groupes |
| Colonne | Poignée de largeur côté canevas (200 à 520 px) ; se vide sans disparaître pendant un déplacement |
| Panneaux | Tous montés (l'état de chaque onglet est gardé) ; pied de panneau (`.panel-foot`) pour les actions et le compte ; recherche en tête (`.panel-search`). Panneau CMS fermé par défaut (vue CMS et panneau Pages) ; un panneau rouvert reprend sa place auprès de ses voisins de la disposition par défaut |
| Pages | Arbre (`PageTree`, aussi dans le menu de la barre d'outils) : pages fixes (dossiers d'après l'adresse), une ligne par collection (le modèle, icône base de données). Repliée, la collection du contenu affiché est en évidence (`--selection`) et ne montre que ce contenu ; dépliée, tous ses contenus ; design propre marqué d'une icône bleue (`--set-text`) ; design sans contenu en orange, supprimable |
| Modèle, design propre | Pastille de la barre d'outils : « Modèle · Projets » (les modifications valent pour tous les contenus) + bouton « Créer un design propre » ; « Design propre » (bleu) + bouton « Revenir au modèle ». Annulable en une étape |
| Variables | Affectations `{% set %}` de la page, de ses gabarits et de ses composants, rangées par propriétaire (dossiers repliables) : nom en chasse fixe, valeur calculée à droite (atténuée : vide, liste, objet, non calculée), icône de boucle ou de condition à gauche, icône « varie » à droite ; clic : expression modifiable (Entrée applique, Échap abandonne), syntaxe vérifiée avant l'application |
| Médias | Filtre par type et tri (récents, anciens, nom, type, taille) en deux petites listes sous la recherche ; dossiers en lignes puis grille de vignettes (icône et extension pour les fichiers non image) ; fil d'Ariane dans un dossier ; glisser une vignette sur un dossier ou le fil d'Ariane pour la ranger, sur la page pour la placer ; pied : nombre, glissière courte de taille des vignettes, nouveau dossier, import (icônes seules) ; fichiers du système déposés sur le panneau (contour d'accent) ; clic : détails par-dessus la colonne (aperçu, adresse, taille, texte alternatif par défaut, Remplacer, Placer) |
| Choisir une image | Fenêtre centrée au-dessus de tout (y compris des volets par-dessus la colonne) : titre, recherche, dossier (dans l'éditeur), vignettes cliquables, nombre et « Importer » en pied ; Échap ou clic hors de la fenêtre : fermée ; ouverte par un bouton icône à droite du champ (source d'image, image de partage, champ image du CMS), désactivé pour une valeur calculée |
| Composants | Arbre de dossiers repliables, nombre d'utilisations ; glisser sur un dossier ou menu contextuel « Déplacer dans … » |
| Dossiers (médias, composants) | Rangement de l'éditeur (`Site::library`) : aucun fichier déplacé, aucune adresse changée ; nouveau dossier (pied du panneau, menu d'un dossier), renommer, supprimer (le contenu remonte d'un niveau) ; nom saisi sur place, refusé (bordure rouge) s'il est vide ou déjà pris |
| Fonction prévue | Bouton désactivé (`.is-planned`), étape en info-bulle ; panneau prévu : `Planned` |
| Raccourcis | Ctrl+\ panneaux ; ↑ ↓ ← → dans les calques ; clic droit : menu contextuel ; voir aussi *Canevas* |
| Ajouter | Familles (Structure, Texte, Médias, CMS, Formulaires, Code) ; tuiles icône + nom, aide en info-bulle ; glisser sur la page ou cliquer ; éléments prévus en pointillés, désactivés |

### Canevas (D-22)

Navigation et glisser-déposer repris de Figma, Framer et Webflow (sources : aide de Figma sur le zoom
https://help.figma.com/hc/en-us/articles/360041065034, raccourcis de sélection de Figma
https://thoughtbot.com/blog/figma-shortcuts-speed-up-your-design-workflow, aide de Framer sur le canevas
https://www.framer.com/help/articles/how-to-use-the-canvas).

| Geste | Effet |
|---|---|
| Molette sur la page | La page défile (comme Webflow) |
| Molette autour du cadre, Maj+molette | La vue se déplace |
| Espace maintenue + glisser, bouton du milieu | La vue se déplace (curseur main) |
| Alt+molette (aussi Ctrl+molette, pincement) | Zoom autour du pointeur (10 à 400 %), sur la page comme autour du cadre ; la hauteur du cadre ne change pas, la page n'est pas remise en page |
| Ctrl+R | Règles en pixels (haut et gauche du canevas) ; menu « Affichage » : règles, guides de colonnes (nombre, gouttière, marge, largeur maximale) en surimpression |
| Changement de point de rupture | Largeur du cadre animée ; contour de l'espace à la couleur d'accentuation choisie |
| Maj+1 / Ctrl+1 | Zoom ajusté ; Maj+0 / Ctrl+0 : 100 % ; Maj+2 / Ctrl+2 : zoom sur la sélection ; Ctrl+plus, Ctrl+moins : paliers. Chiffres lus par touche physique (claviers AZERTY) |
| Clic dans le vide autour du cadre | Plus de sélection |
| Glisser un élément de la page | Déplacement ; Alt : copie. Élément estompé, étiquette qui suit le pointeur |
| Glisser une tuile d'« Ajouter » | Insertion à l'endroit montré |
| Entrée, Maj+Entrée, Tab, Maj+Tab | Premier enfant, parent, frère suivant, précédent (Figma) |
| Alt+↑, Alt+↓ | Monter, descendre parmi les frères |
| Ctrl+C, Ctrl+X, Ctrl+V | Copier, couper, coller un élément (à côté d'un élément de même balise, sinon dedans ou après) ; d'un site à l'autre |
| Ctrl+G, Ctrl+Maj+G | Envelopper (`span` dans un texte, `div` ailleurs), retirer l'enveloppe |
| Clic droit | Menu : parent, zoom, copier, couper, coller, dupliquer, envelopper, monter, descendre, supprimer |

**Emplacement de dépôt** (modèle de Webflow) : sur le bord d'un élément, dans le sens où ses frères se suivent
(horizontal s'ils sont sur la même ligne, ou si le parent est une pile ou une grille en ligne), avant ou après
lui ; au milieu d'un élément qui peut contenir des enfants, dedans, à côté de l'enfant le plus proche. Le
conteneur d'arrivée est entouré en pointillés avec son nom, la ligne d'insertion est bleue. Règles du HTML
respectées : pas de bloc dans un paragraphe ou un titre, pas de lien dans un lien, `li` seulement dans une liste.
Autour de l'emplacement du contenu d'un gabarit, l'élément va dans la page affichée (pas dans le gabarit partagé).
Défilement automatique près des bords haut et bas du cadre ; Échap annule.

## Panneau de styles (D-20)

Public visé : une personne qui ne connaît pas CSS. Chaque contrôle porte un **nom simple** ; le nom CSS est
en info-bulle (et dans la section « Code CSS »). On règle par **gestes** (glisser, cliquer une icône, choisir une
couleur) ; la saisie au clavier reste possible partout pour qui la préfère.

### Choix retenus, fonction par fonction

| Fonction | Retenu | Repris de | Pourquoi |
|---|---|---|---|
| Cible | Classes en pastilles dans un champ (clic : classe réglée, × : retirée, saisie : ajoutée ; classes combinées en italique), état (Normal, Survol, Appui, Focus clavier), point de rupture et nombre d'éléments qui portent la classe | Webflow [W1][W16] | seul modèle fidèle à une vraie cascade CSS |
| Origine d'une valeur | Libellé sur fond bleu : réglé ici ; orange : hérité d'un écran plus large ou de l'état de base (D-21) ; sinon gris (valeur effective en indication). Clic sur le libellé : origine et bouton de réinitialisation (flèche) ; Alt+clic : réinitialise | Webflow [W1][W18], Framer [F5] | code connu des utilisateurs de Webflow |
| Nombres | Glisser le libellé (Maj ×10, Alt ×0,1) ; ↑/↓ ±1, Maj ±10, Alt ±0,1 ; calculs `12+4`, `*2` ; unité tapée ou choisie dans un menu court (`px`, `%`… ; explication en info-bulle) ; champ vide = valeur retirée | Figma [G5], Webflow [W15], Framer [F1] | glisser sans touche spéciale se découvre seul |
| Couleur | Pastille + valeur ; panneau : plan saturation-luminosité, curseur de teinte, curseur d'opacité, pipette (si le système la permet), modèle Hex / RVB / TSL / TSV, couleurs du site (variables d'abord, puis couleurs utilisées), contraste du texte (AA/AAA) | Figma [G7][G8], Webflow [W11][W19] | aucune valeur à taper |
| Espacement | Boîte symétrique (rangées de même hauteur) : « Extérieur » autour d'« Intérieur », légendes dans l'angle ; chaque côté se glisse ou se tape ; Alt = côté opposé aussi, Maj = les quatre côtés ; aide en info-bulle | Webflow [W2], Figma [G2] | seul outil avec la marge extérieure ; modificateurs communs aux deux |
| Disposition | Bloc / Pile / Grille / Masqué (+ variantes « en ligne ») ; pile : direction à icônes, inverser, boîte d'alignement 3 × 3 et deux listes en mots (Horizontal, Vertical) selon la direction, écart, retour à la ligne ; grille : nombre de colonnes | Framer (« Stack ») [F2], Figma [G3], Webflow [W4][W5] | mots et icônes au lieu de `justify-content` |
| Élément dans une pile | Rétrécir si nécessaire / Grandir si possible / Ni grandir ni rétrécir, écrits `flex: 0 1 auto`, `1 1 auto`, `0 0 auto` (valeurs `initial`, `auto`, `none` de CSS, dont les descriptions sont celles de Webflow, qui ne publie pas ses valeurs [M1]) | Webflow [W4] | phrases plutôt que `flex: 1 1 auto` |
| Taille | Largeur, hauteur, minimum, maximum ; menu d'unités expliqué (px fixe, % du parent, vw/vh de l'écran, Auto, Ajusté au contenu) ; débordement en mots ; ajustement des images (Remplir, Ajuster, Étirer…) ; proportions nommées | Framer [F1], Figma [G2][G11], Webflow [W7] | modes nommés par leur effet |
| Position | Normale / Relative / Absolue / Fixe / Collante avec une phrase « par rapport à quoi » ; décalages dans une boîte ; préréglages (Tout le parent, coins, bords) ; ordre de superposition | Framer [F3], Webflow [W8] | l'effet est dit en clair |
| Texte | Police (polices du site), graisse nommée (Fin … Noir), taille, interligne, espacement des lettres, couleur, alignement et casse à icônes, italique / souligné / barré | Figma [G12], Webflow [W9] | noms de graisse plutôt que 100…900 |
| Fond | Couleur ; image : Remplir (cover) / Ajuster (contain) / Auto, position 3 × 3, répétition | Figma [G11], Webflow [W10] | |
| Bordure | Arrondi en un champ, « Angles indépendants » pour quatre ; côtés (tous, haut, droite, bas, gauche), épaisseur, style à icônes, couleur | Figma [G15], Webflow [W12] | |
| Effets | Opacité (curseur), ombres en liste (portée / intérieure, X, Y, flou, étendue, couleur), curseur de la souris | Figma [G13], Webflow [W13] | |
| Code CSS | Toutes les déclarations de la règle, `!important`, ajout libre ; section repliée | — | rien n'est perdu pour les utilisateurs avancés |

Pendant un geste (glisser, couleur), la page montre le résultat en direct sans écrire le fichier ; relâcher
enregistre **une seule** modification (une seule étape d'annulation).

Différé : éditeur de dégradés (Figma [G10]), filtres et flou, transitions et transformations (étape 6) — modifiables
dans « Code CSS » d'ici là.

### Noms affichés

| CSS | Français | Anglais |
|---|---|---|
| `display` | Affichage : Bloc, Pile, Grille, Masqué | Display: Block, Stack, Grid, Hidden |
| `flex-direction` | Direction : Horizontale, Verticale, Inverser | Direction: Horizontal, Vertical, Reverse |
| `justify-content`, `align-items` | Horizontal / Vertical : Gauche, Centre, Droite, Haut, Bas, Étirés, Ligne de base, Espacés (entre, autour, égaux) | Horizontal / Vertical: Left, Center, Right, Top, Bottom, Stretch, Baseline, Space between / around / evenly |
| `gap` | Écart | Gap |
| `flex-wrap` | Retour à la ligne | Wrap |
| `margin`, `padding` | Extérieur, Intérieur (« Marge extérieure / intérieure » en info-bulle) | Margin, Padding |
| `width`, `min-width`… | Largeur, Largeur min., Largeur max. | Width, Min width, Max width |
| `overflow` | Débordement : Visible, Masqué, Défiler, Auto | Overflow: Visible, Hidden, Scroll, Auto |
| `object-fit` | Ajustement : Remplir, Ajuster, Étirer, Aucun | Fit: Fill, Fit, Stretch, None |
| `position` | Position : Normale, Relative, Absolue, Fixe, Collante | Position: Static, Relative, Absolute, Fixed, Sticky |
| `z-index` | Superposition | Layer order |
| `font-weight` | Graisse : Fin, Très léger, Léger, Normal, Moyen, Demi-gras, Gras, Très gras, Noir | Weight: Thin … Black |
| `line-height`, `letter-spacing` | Interligne, Espacement | Line height, Letter spacing |
| `text-transform` | Casse | Case |
| `background-*` | Fond | Background |
| `border-radius` | Arrondi | Corner radius |
| `box-shadow` | Ombres : portée, intérieure ; Flou, Étendue | Shadows: drop, inner; Blur, Spread |
| `cursor` | Curseur | Cursor |

Les libellés tiennent dans leur colonne en français et en anglais (contrôlé par `tools/editor-e2e.mjs`).
Pas de texte explicatif affiché dans les panneaux : les aides sont en info-bulles (D-21).

Sources (lues le 14/10/2026) :
[W1] https://help.webflow.com/hc/en-us/articles/33961362040723 ·
[W2] …/33961243177875 · [W4] …/33961260795155 · [W5] …/33961238138643 · [W7] …/33961381659411 ·
[W8] …/33961228002963 · [W9] …/33961334261779 · [W10] …/33961278898323 · [W11] …/33961288849171 ·
[W12] …/33961270594067 · [W13] …/33961245810451 · [W15] …/33961290465043 · [W16] …/33961301727251 ·
[W18] …/33961332425875 · [W19] …/33961268146323 ·
[F1] https://www.framer.com/academy/lessons/sizing-modes · [F2] …/frames-vs-stacks ·
[F3] …/layer-positioning-modes-in-framer · [F5] …/component-variants-in-framer ·
[G2] https://help.figma.com/hc/en-us/articles/360040451373 · [G3] …/31289464393751 · [G5] …/360039956914 ·
[G7] …/360041003774 · [G8] …/360043042113 · [G10] …/34208860210199 · [G11] …/360041098433 ·
[G12] …/360039956634 · [G13] …/360041488473 · [G15] …/360050986854 ·
[M1] https://developer.mozilla.org/en-US/docs/Web/CSS/flex (copie lue : https://docs.w3cub.com/css/flex)

## Textes

- Les phrases viennent de `revel-i18n` (Fluent), jamais du cœur : un échec est une clé (`error-…`) et des
  paramètres (D-16).
- Les nombres sont passés bruts ; Fluent les met en forme (`4 356`, `716,5` en français ; `4,356`, `716.5` en
  anglais) et choisit le pluriel (`1 fichier`, `2 fichiers`). Les cas à zéro ont leur propre phrase
  (« le site en ligne est déjà à jour »).
- Dates : format local de la langue (`13 oct. 2026, 15:11`).

## Vérification

`tools/ui-shots.mjs` produit des captures de chaque écran (sombre, clair, français, anglais, avec et sans
accent) à partir de l'interface servie avec une API simulée (`apps/revel-ui/dev/mock-tauri.js`).
Chaque évolution d'interface est relue sur ces captures avant d'être livrée.

```sh
cd apps/revel-ui && trunk build && (cd dist && python3 -m http.server 8131 &)
NODE_PATH=$(npm root -g) node tools/ui-shots.mjs http://127.0.0.1:8131/ captures/
```
