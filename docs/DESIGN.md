# Système de design de Revel

L'interface fait partie du produit au même titre que le moteur. Ce document est la référence :
tout écran nouveau est construit avec ces jetons et ces composants ; si un besoin n'est pas couvert,
on étend d'abord le système (ici et dans `apps/revel-ui/styles/`), puis on l'utilise.

Références d'usage : Figma (densité, panneaux, contrôles compacts), Framer et Webflow
(structure d'un outil de création de sites : barre supérieure, panneau gauche, canevas, inspecteur).
Revel n'en reprend ni les couleurs ni les marques.

## Principes

1. **Le contenu de l'utilisateur est la seule couleur.** L'interface est en gris neutres.
   Aucune couleur décorative ; les couleurs fonctionnelles (erreur, réussite) restent discrètes.
2. **Un seul accent, choisi par l'utilisateur.** *Réglages > Couleur d'accent* : « Aucune » par défaut.
   Sans accent, l'action principale est en contraste inversé (blanc sur sombre, noir sur clair).
   L'accent ne sert qu'à trois choses : action principale, sélection, focus.
3. **Une action principale par zone.** Une seule `.btn--primary` visible par section ou barre.
4. **Densité d'outil.** Texte 13 px, contrôles 28 px, grille de 4 px : beaucoup d'information sans
   surcharge, comme Figma. Les pages de réglages restent aérées.
5. **Mots simples, verbes d'action.** Un bouton dit ce qu'il fait (« Générer le site », « Publier ») ;
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

Icônes : `apps/revel-ui/src/icons.rs`, 16 px, trait de 1,5 px, extrémités arrondies, dessinées pour Revel.

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
