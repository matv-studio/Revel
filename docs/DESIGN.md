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
| Structure | `--topbar-h` 44, `--sidebar-w` 240, `--inspector-w` 280 | Coque de l'application |
| Mouvement | `--ease`, `--fast` 120 ms, `--normal` 200 ms | Transitions de retour d'action |

Thèmes : sombre (défaut), clair, ou système (`data-theme` absent sur `<html>`).
L'accent est appliqué par l'application en définissant `--accent` et `--accent-ink` sur `:root`
(`--accent-ink` est calculé : noir ou blanc selon la luminance de l'accent).

**Police** : Geist (variable, licence OFL, fichiers dans `apps/revel-ui/fonts/`), intégrée pour un
rendu identique sous Windows, macOS et Linux. Police à chasse fixe du système (`--font-mono`) uniquement pour
les valeurs techniques à comparer ou recopier (empreinte d'un serveur).

## Structure (`styles/shell.css`)

```
┌ barre supérieure 44 px : logo · fil d'Ariane ·················· actions de la page ┐
├ panneau gauche 240 px ─┬ zone principale ──────────────────────┬ inspecteur 280 px ┤
│ Tous les sites          │ page centrée (960 px max)             │ (étape 2 :        │
│ Récents                 │   ou                                  │  propriétés de    │
│   Site ouvert           │ canevas plein (étape 2 : éditeur)     │  l'élément        │
│     Générer · Publier   │                                       │  sélectionné)     │
│ ─────────               │                                       │                   │
│ Réglages                │                                       │                   │
└─────────────────────────┴───────────────────────────────────────┴───────────────────┘
```

- La **barre supérieure** porte l'activité en cours : une ligne de 2 px sous la barre (génération,
  publication), visible depuis n'importe quel écran.
- À l'étape 2, le panneau gauche affichera pages et calques du site ouvert ; l'inspecteur apparaît à droite.
- Sous 760 px de large, le panneau gauche est masqué (fenêtres étroites).

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
| Sites | `.site-grid` > `.site-card` (vignette à monogramme, nom, date ; actions au survol) ou `.table` | Accueil : grille ou liste, recherche ; « Ouvrir l'éditeur » direct |
| Tableau | `.table`, `.row-link`, `.table--compact` | Listes de sites, de collections ; ligne entière cliquable |
| Faits | `.facts`, `.facts--inline` | Chiffres clés ; paires libellé / valeur |
| Puce | `.chip` | Étiquette courte (environnement) |
| Liste | `.list-plain` (`.is-blocking`) | Avertissements, points à vérifier |
| Retour | `.notice--error`, `.status--ok`, `.empty` | Erreur (avec action de correction si elle existe), réussite, état vide (avec l'action qui le remplit) |
| Valeur technique | `.code`, `.code--large` | Empreinte, clé : chasse fixe, sélection en un clic ; `--large` pour un code à recopier (autorisation GitHub) |
| Compte relié | `.account`, `account.rs` | État de connexion et action (connecter, déconnecter) ; le même composant dans Réglages et dans un formulaire |
| Sous-bloc | `.subsection` | Groupe distinct à l'intérieur d'une section (compte requis par un formulaire) |
| Boîte de dialogue | `.overlay` > `.dialog` (`dialog.rs`) | Confirmation ou saisie ; remplace `confirm()`/`prompt()` du système ; Échap ferme ; action destructrice : focus sur « Annuler » |
| Navigation | `.nav-item`, `.nav-item--sub`, `.is-active` | Panneau gauche |
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
┌ barre d'outils : page · points de rupture · zoom · scripts ········· panneaux ? annuler rétablir ┐
├ colonne gauche ────────┬ canevas (fixe) ────────────────────────────┬ colonne droite ───────────┤
│ Pages Composants Médias │                                            │ Styles Réglages Interactions│
│ ─────────────────────── │        page à la largeur du point          │                            │
│ Calques CMS Ajouter     │        de rupture, zoom                    │                            │
│   (recherche, arbre,    │                                            │                            │
│    actions en bas)      │   [fenêtre flottante possible]             │                            │
└ fil d'Ariane de l'élément sélectionné · propriétaire ········· enregistrement · largeur ──────────┘
```

| Élément | Règles |
|---|---|
| Onglet (`.dock-tab`) | Texte seul ; actif en relief (fond, contour) ; glisser pour déplacer, clic pour afficher |
| Dépôt | Bande d'onglets : rejoint le groupe ; haut ou bas d'un panneau : nouveau groupe au-dessus ou au-dessous ; colonne vide : nouveau groupe ; canevas : fenêtre flottante. Zone de dépôt en bleu |
| Groupe | Menu ⋯ : liste des onglets, détacher, fermer ; poignée de hauteur entre deux groupes |
| Colonne | Poignée de largeur côté canevas (200 à 520 px) ; se vide sans disparaître pendant un déplacement |
| Panneaux | Tous montés (l'état de chaque onglet est gardé) ; pied de panneau (`.panel-foot`) pour les actions et le compte ; recherche en tête (`.panel-search`) |
| Fonction prévue | Bouton désactivé (`.is-planned`), étape en info-bulle ; panneau prévu : `Planned` |
| Raccourcis | Ctrl+\ panneaux ; Ctrl+0 zoom ajusté, Ctrl+1 100 %, Ctrl+molette ; ↑ ↓ ← → dans les calques ; clic droit : menu contextuel |

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
