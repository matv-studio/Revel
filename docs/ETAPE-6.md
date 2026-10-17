# Étape 6 — Animations et interactions, empaquetage, mises à jour, signature, documentation

Objectif : animer un site sans écrire de code ni passer par After Effects, aussi simplement que dans Framer —
une liste d'animations prêtes à l'emploi, chacune avec quelques réglages — puis lire les animations Lottie
exportées d'After Effects pour ceux qui animent à la main ; faire ensuite une passe de comparaison avec
Framer, Figma et Webflow (demande du 16/10/2026 : juste après les animations et transitions) ; enfin empaqueter
Revel pour la version 1.0 (mises à jour automatiques, signature, documentation, installateur léger).

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
5. Passe de comparaison avec Framer, Figma et Webflow et passe complète d'expérience, d'interface et de
   direction artistique, juste après les transitions entre les pages.
6. Mises à jour automatiques de Revel, installateurs signés, documentation intégrée ; installateur 1.0 léger,
   au logo de Revel, sans langue de traduction livrée (D-46).

## Sous-étapes

| N | Contenu | Modules |
|---|---|---|
| 6.1 | **Animations prêtes à l'emploi** (demande du 16/10/2026, à l'image de Framer) : catalogue ci-dessous, panneau Interactions, aperçu dans le canevas, script d'animation produit à la génération | `revel-render`, `revel-core`, `revel-ui` |
| 6.2 | **Animations Lottie** (After Effects avec Bodymovin ou LottieFiles) : bloc « Animation Lottie », lecteur hébergé avec le site, réglages de lecture | `revel-assets`, `revel-render`, `revel-ui` |
| 6.3 | **Interactions entre éléments** : déclencheurs (clic, survol, arrivée à l'écran, défilement, chargement) et actions sur d'autres éléments (afficher, masquer, classe, animation) ; menu, onglets, accordéon prêts à l'emploi | `revel-render`, `revel-ui` |
| 6.4 | **Transitions entre les pages** (fondu, glissement) | `revel-render`, `revel-ui` |
| 6.5 | **Passe de comparaison** avec Framer, Figma et Webflow ; passe d'expérience, d'interface et de direction artistique (placée juste après 6.4 le 16/10/2026) | tous |
| 6.6 | **Mises à jour automatiques** de Revel (vérifiées et signées) | `revel-app` |
| 6.7 | **Signature du code** Windows et macOS (D-09) : déplacée à l'étape 8, fonctions optionnelles (17/10/2026) | intégration continue |
| 6.8 | **Documentation** : aide intégrée, premiers pas | `revel-ui`, `docs/` |
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
| 6.2 | Terminée : bloc « Animation Lottie » (Ajouter), fichier `.json` ou `.lottie` des médias, lecture à l'écran, au survol, à l'appui ou au défilement, boucle, vitesse ; lecteur lottie-web hébergé avec le site, visible dans le canevas (D-74) |
| 6.3 | Terminée : rubrique « Actions » du panneau Interactions (déclencheur, action, élément visé), onglets et accordéon prêts à l'emploi, script produit seulement s'il sert (D-75) |
| 6.4 | Terminée : transition entre les pages (fondu, glissement) dans Paramètres › Général, en CSS, coupée pour le mouvement réduit (D-76) |
| 6.5 | Terminée (17/10/2026) : passe d'écran en écran (éditeur, Ajouter, Styles, Interactions, CMS, Localisation, Paramètres, accueil, Réglages, aide), en thème sombre et clair, comparée à Webflow, Framer et Figma ; corrigé : exemple du titre de l'onglet sans le nom du site quand les données sont dans le document, icône propre aux onglets, tuile « Lottie » à la hauteur des autres, « Tronquer » au lieu d'un libellé coupé, nombre de contenus (et non d'« entrées ») dans le CMS, « Adresse de la page » sans terme technique, champ robots.txt à sa taille, page de l'éditeur gardée (sans rechargement) après un réglage sans effet sur elle dans Paramètres (transition, référencement, formulaires, redirections) ; essais de bout en bout ajoutés (onglets, accordéon, Lottie, actions, transition, aide) |
| 6.6 | Terminée (17/10/2026, D-77) : recherche au lancement, Réglages › Mises à jour, installation vérifiée et relance ; clé déclarée par l'utilisateur, active depuis la 0.6.0 : `latest.json` et une signature par installateur (11 cibles), signature du `.deb` vérifiée avec la clé publique intégrée à l'application ; première mise à jour proposée quand une version plus récente sera publiée (ni brouillon ni préversion), ou dès sa publication en préversion pour qui a coché Réglages › Mises à jour › « Recevoir les préversions » (D-86) |
| 6.7 | Déplacée à l'étape 8 (8.1, fonctions optionnelles) le 17/10/2026 ; la construction des installateurs reste prête (D-09) |
| 6.8 | Terminée : aide intégrée « Premiers pas » (menu « ? » de l'éditeur), quatorze rubriques en français et en anglais (`apps/revel-ui/help/`), libellés vérifiés sur ceux de l'interface |
| 6.9 | Terminée : installateur Windows au logo de Revel (bandeau, barre latérale, icône, français et anglais selon le système), image disque macOS avec fond et emplacements ; langues de traduction non livrées dès la 0.5.1 (D-79, avancé sur D-46) ; vérifié sur la 0.6.0 : 14 à 20 Mo par installateur (99 Mo pour l'AppImage) au lieu de 256 à 355 Mo ; `.msi` et `.rpm` gardés comme installateurs de remplacement (D-87 annulée) |

## Passe complète du 16/10/2026 (après 6.1)

Demande de l'utilisateur, faite sur le site Alunites importé : éliminer les incohérences de l'application, et
recréer le site depuis l'application seule pour prouver que les outils suffisent sans écrire de code.

| Point | État |
|---|---|
| Polices et couleurs nommées (« Montserrat », « Accent ») au lieu de `var(--font)` ; menus déroulants alignés | Fait |
| Espacements : poignées au milieu des côtés, teinte unie sans hachures, écarts entre enfants (D-60) | Fait |
| Netteté du canevas dézoomé (D-63) | Fait (à confirmer sous Windows et macOS) |
| Calques : liste nommée, conditions en marque, liens d'évitement absents, texte rendu au lieu des `{{ … }}`, icône de texte, double-clic pour modifier (D-61) | Fait |
| Double-clic sur un texte riche (HTML venu des données) dans la page : modification avec sa mise en forme | Fait |
| Champ « Texte » en tête du panneau de styles | Fait |
| Classes, animations, attributs posés sans recharger la page ; bouton « scripts du site » retiré (D-62) | Fait |
| Pages rangées en les glissant, sans effet sur les adresses (D-64) | Fait |
| Largeur du cadre : la vue suit le pointeur au bord de la zone | Fait |
| Réglages de l'application sur une page (D-65) | Fait |
| Sélection depuis les calques avec tous les écrans affichés : un seul écran visé (D-66) | Fait |
| Aperçu intégré bloqué sous Windows (D-67) | Fait (à confirmer sous Windows) |
| Composants : canevas propre avec variantes (survol, appui, personnalisées), transition, variante choisie par utilisation (D-68) | Fait |
| Animer un composant sans l'envelopper ; onglet Réglages fondu dans Styles, « Paramètres avancés » (D-69) | Fait |
| Variables du site lisibles dans les champs de taille (« Step 3 » au lieu de `var(--step-3)`) | Fait |
| Règle de style créée deux fois (Entrée puis sortie du champ) | Corrigé |
| Canevas libre ou vue verrouillée, bouton de la barre (D-70) | Fait |
| Couleurs du site créées depuis le sélecteur ; paramètres du site en six rubriques (D-71) | Fait |
| Mise en forme dans le texte : barre gras, italique, lien, span (D-72) | Fait |
| Accueil et modèle d'un projet recréés depuis l'application seule (17/10/2026) | Fait |
| Texte mis en forme modifié d'un bloc, sur la page et dans le panneau (D-72) | Fait |
| Grille, écarts, place d'un enfant, filtres, transformations, transitions, mélange, retour à la ligne (D-73) | Fait |
| CMS : écriture des dates, texte avant et après une valeur, affichage selon un champ, image de fond reliée, liste sans le contenu affiché, liste d'un champ, dossiers de contenus proposés (D-73) | Fait |
| Site Alunites entier recréé depuis l'application seule | Dernière étape (demande du 17/10/2026) |

## Retours sur la 0.5.0 (17/10/2026)

| Point | État |
|---|---|
| Canevas mis à jour sur place après une modification, sans rejouer les animations (D-78) | Fait |
| Listes déroulantes à la charte (filtre des médias et toutes les autres) | Fait (Chromium 135 et plus, WebView2) |
| Composant non déplié dans les calques, ouvert dans son espace par un double-clic | Fait |
| Disposition à la Webflow : Bloc, Flex, Grille, Masqué et autres affichages, alignement, écarts, grille par nombre de pistes, enfant de grille ou flex (D-78) | Fait |
| Fichiers de la recréation d'Alunites remis en l'état | Fait |
| Ajouter : un clic n'ajoute rien, double-clic ajoute ; conteneur vide montré par son icône (50 px) | Fait |
| Supprimer un site depuis l'accueil (corbeille du système) | Fait |
| Barres de défilement fines | Fait |
| Boutons « Dupliquer » et « Supprimer » centrés dans les paramètres de la page | Fait |
| Menu du site gardé dans les Réglages, retour au projet | Fait |
| Langues de traduction : installées seulement (« Aucune langue installée » sinon), les autres sous « Plus de langues » | Fait |
| Aucune langue de traduction livrée dans les versions construites (D-79) | Fait (dès la 0.5.1) |
