# Étape 6 — Animations et interactions, empaquetage, mises à jour, signature, documentation

Objectif : animer un site sans écrire de code ni passer par After Effects, aussi simplement que dans Framer —
une liste d'animations prêtes à l'emploi, chacune avec quelques réglages — puis lire les animations Lottie
exportées d'After Effects pour ceux qui animent à la main ; ensuite empaqueter Revel pour la version 1.0
(mises à jour automatiques, signature, documentation, installateur léger) après une dernière passe de
comparaison avec Framer, Figma et Webflow.

**Critères d'acceptation**
1. Panneau Interactions : animations prêtes à l'emploi rangées par moment (apparition, texte, survol, appui,
   boucle, défilement), choisies dans une grille avec aperçu au survol, réglées en quelques champs (durée,
   délai, courbe, distance ou intensité, décalage entre éléments, déclencheur), essayées dans le canevas d'un
   clic.
2. Site publié : animations fluides sans bibliothèque extérieure (un script de quelques kilo-octets, produit
   seulement si le site en utilise), contenu visible sans JavaScript, animations coupées pour les visiteurs qui
   demandent moins de mouvement (`prefers-reduced-motion`).
3. Animations Lottie exportées d'After Effects : fichier ajouté aux médias, lecteur hébergé avec le site (pas de
   service extérieur), lecture automatique, en boucle, au survol ou au défilement.
4. Interactions entre éléments (clic sur un bouton qui ouvre un menu, onglets, accordéons) sans code.
5. Mises à jour automatiques de Revel, installateurs signés, documentation intégrée.
6. Fin d'étape : passe de comparaison avec Framer, Figma et Webflow et passe complète d'expérience, d'interface
   et de direction artistique ; installateur 1.0 léger, au logo de Revel, sans langue de traduction livrée
   (D-46).

## Sous-étapes

| N | Contenu | Modules |
|---|---|---|
| 6.1 | **Animations prêtes à l'emploi** (demande du 16/10/2026, à l'image de Framer) : catalogue ci-dessous, panneau Interactions, aperçu dans le canevas, script d'animation produit à la génération | `revel-render`, `revel-core`, `revel-ui` |
| 6.2 | **Animations Lottie** (After Effects avec Bodymovin ou LottieFiles) : bloc « Animation Lottie », lecteur hébergé avec le site, réglages de lecture | `revel-assets`, `revel-render`, `revel-ui` |
| 6.3 | **Interactions entre éléments** : déclencheurs (clic, survol, arrivée à l'écran, défilement, chargement) et actions sur d'autres éléments (afficher, masquer, classe, animation) ; menu, onglets, accordéon prêts à l'emploi | `revel-render`, `revel-ui` |
| 6.4 | **Transitions entre les pages** (fondu, glissement) | `revel-render`, `revel-ui` |
| 6.5 | **Mises à jour automatiques** de Revel (vérifiées et signées) | `revel-app` |
| 6.6 | **Signature du code** Windows et macOS (D-09) | intégration continue |
| 6.7 | **Documentation** : aide intégrée, premiers pas | `revel-ui`, `docs/` |
| 6.8 | **Passe de comparaison** avec Framer, Figma et Webflow ; passe d'expérience, d'interface et de direction artistique | tous |
| 6.9 | **Installateur 1.0** léger, au logo de Revel, sans langue de traduction livrée (D-46) | `revel-app`, intégration continue |

## Catalogue des animations prêtes à l'emploi (6.1)

Chaque élément peut recevoir une animation par moment. Réglages communs : **durée** (secondes), **délai**
(secondes), **courbe** (Douce, Rebond, Régulière, Linéaire). Valeurs par défaut entre parenthèses.

**Apparition** — quand l'élément arrive à l'écran ou au chargement de la page. Réglages : déclencheur (À
l'écran, Au chargement), durée (0,6 s), délai (0 s), courbe (Douce), distance (32 px, glissements), décalage
entre les éléments contenus (0 s : l'élément entier ; sinon ses enfants l'un après l'autre), rejouer à chaque
passage (non).

| Animation | Effet |
|---|---|
| Fondu | Opacité 0 → 1 |
| Monter | Arrive d'en dessous |
| Descendre | Arrive d'au-dessus |
| Depuis la gauche | Arrive de la gauche |
| Depuis la droite | Arrive de la droite |
| Zoom avant | Agrandissement de 90 % à 100 % |
| Zoom arrière | Réduction de 110 % à 100 % |
| Flou | Netteté progressive (flou de 12 px) |
| Rebond | Apparition de 50 % à 100 % avec léger dépassement |
| Basculer | Rotation 3D vers l'avant |
| Tourner | Rotation légère en montant |
| Dévoiler | Révélé de bas en haut (rideau) |

**Texte** — lettres, mots ou lignes l'un après l'autre. Réglages : découpe (Lettres, Mots, Lignes), déclencheur,
durée par morceau (0,5 s), décalage (0,03 s par lettre, 0,06 s par mot, 0,12 s par ligne), délai, courbe,
rejouer.

| Animation | Effet |
|---|---|
| Fondu | Chaque morceau apparaît |
| Monter | Chaque morceau monte en apparaissant |
| Flou | Chaque morceau devient net |
| Rebond | Chaque morceau grandit avec dépassement |
| Basculer | Chaque morceau bascule en 3D |
| Machine à écrire | Lettres affichées une à une, sans transition |

**Survol** — pendant que le pointeur est sur l'élément (ordinateurs). Réglages : durée (0,25 s), courbe,
intensité (100 %).

| Animation | Effet |
|---|---|
| Agrandir | Échelle 105 % |
| Réduire | Échelle 95 % |
| Soulever | Monte de 6 px avec une ombre |
| Ombre | Ombre portée plus marquée |
| Estomper | Opacité 70 % |
| Incliner | Rotation de 3° |
| Éclaircir | Luminosité 115 % |
| Souligner | Trait qui se dessine sous le texte |

**Appui** — pendant le clic ou le toucher. Réglages : durée (0,12 s), intensité.

| Animation | Effet |
|---|---|
| Enfoncer | Échelle 95 % |
| Écraser | Aplati puis relâché |
| Pousser | Descend de 2 px |

**Boucle** — en continu, mise en pause hors de l'écran. Réglages : durée d'un cycle (2 s), délai, intensité,
aller-retour ; bandeau : sens et pause au survol.

| Animation | Effet |
|---|---|
| Rotation | Tour complet régulier |
| Pulsation | Grandit et revient (106 %) |
| Flottement | Monte et descend de 10 px |
| Balancement | Oscille de ±6° |
| Secousse | Tremble de gauche à droite |
| Clignotement | Opacité 100 % ↔ 30 % |
| Rebondir | Saute et retombe |
| Battement | Double pulsation de cœur |
| Bandeau défilant | Contenu qui défile sans fin (logos, mots), comme le Ticker de Framer |

**Défilement** — suit la position de l'élément dans la fenêtre pendant le défilement. Réglage : intensité.

| Animation | Effet |
|---|---|
| Parallaxe | Se déplace moins vite que la page (80 px) |
| Fondu en entrant | Apparaît en entrant dans la fenêtre |
| Zoom en entrant | Grandit de 85 % à 100 % en entrant |
| Rotation | Tourne en traversant la fenêtre (±10°) |
| Glissement horizontal | Traverse la fenêtre de droite à gauche (120 px) |
| Disparaître en sortant | S'estompe en sortant par le haut |

Repères : effets Appear, Hover, Press, Loop, Text Effects, Scroll Speed et Scroll Transform de Framer
([Framer Academy](https://www.framer.com/academy/lessons/framer-animations-overview),
[Text Effects](https://framer.com/updates/text-effects)).

## Avancement

| N | État |
|---|---|
| 6.1 | Terminée : panneau Interactions (six moments, grille de 44 animations dont la vignette s'anime au survol, réglages par animation, aperçu dans le canevas, retrait) ; attribut `data-revel-*` par moment ; script du site produit si une page est animée (contenu visible sans JavaScript, mouvement réduit respecté, texte découpé au chargement et gardé pour les lecteurs d'écran) ; éclair sur les éléments animés dans les calques ; images et composants animés par un bloc qui les entoure ; animations décrites aux assistants IA (D-59) |
| 6.2 | À venir |
| 6.3 | À venir |
| 6.4 | À venir |
| 6.5 | À venir |
| 6.6 | À venir |
| 6.7 | À venir |
| 6.8 | À venir |
| 6.9 | À venir |
