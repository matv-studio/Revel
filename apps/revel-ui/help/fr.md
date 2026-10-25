# Premiers pas

Revel construit un site sans écrire de code : vous placez des éléments sur la page, vous les réglez dans les panneaux, puis vous publiez. Tout se fait sur votre ordinateur ; rien n'est envoyé ailleurs sans votre accord.

## Créer ou ouvrir un site

1. Sur l'accueil de Revel, choisissez **Nouveau site**.
2. Donnez-lui un nom et choisissez un modèle : **Vide**, **Vitrine** ou **Blog**.
3. Revel ouvre l'**Éditeur**. Vos sites récents restent sur l'accueil.

Un site Revel s'ouvre avec **Ouvrir un site**. Un site existant se reprend avec **Importer** :

- **Dossier** : un dossier de pages HTML (fait à la main, ou export du code de Webflow décompressé) ou un projet Eleventy. Chaque page garde son adresse ; l'en-tête et le pied communs vont dans un gabarit, les styles dans le panneau Styles. Export Webflow : ajoutez les fichiers CSV des collections (CMS › collection › Export) ou un jeton de l'API de Webflow pour reprendre les contenus du CMS.
- **Lien** : l'adresse d'un site en ligne (fait avec Framer, Webflow ou autre). Revel lit ses pages et copie ses images, polices et styles dans le dossier des projets ; les animations d'apparition deviennent des animations Revel.
- **Dépôt Git** : collez l'adresse d'un dépôt (`https://github.com/compte/site`, ou celle d'un dossier : `…/tree/main/docs`). Revel le copie dans le dossier des projets, historique compris, puis l'importe. Un site à construire (SvelteKit, Next, Astro, Hugo, Jekyll…) peut être construit par Revel (**Construire** : les outils nécessaires sont téléchargés une fois, puis gardés dans **Réglages › Outils de construction**) ; vous pouvez aussi importer son site publié ou le dossier produit par sa construction. Le dépôt de votre équipe dans un dossier partagé se choisit avec **Dossier…**.
- **Figma** : le lien du fichier (Partager › Copier le lien) et un jeton personnel (Figma › Settings › Security › Personal access tokens). Choisissez les cadres : chacun devient une page, le premier est l'accueil.
- **WordPress** : le fichier d'export (Outils › Exporter › Tout le contenu). Les articles deviennent une collection, les pages gardent leur adresse, les images sont copiées depuis le site.

À la fin, la boîte montre ce qui a été repris et les points à vérifier, puis ouvre le site dans l'éditeur.

## L'éditeur

- **Au centre, le canevas** : la page telle qu'elle sera publiée. Un clic sélectionne un élément ; Maj+clic en ajoute un autre à la sélection.
- **À gauche** : **Pages** (pages du site et pages des collections), **Composants**, **Médias**, puis **Calques** (l'arbre de la page) et **Ajouter**.
- **À droite** : **Styles** (apparence de l'élément choisi) et **Interactions** (animations et actions).
- **En haut** : les écrans (ordinateur, tablette, téléphone), le zoom, l'aperçu et **Publier**.

Les panneaux se déplacent : glissez un onglet vers une autre colonne ou sur le canevas pour en faire une fenêtre.

## Ajouter des éléments

1. Sélectionnez l'endroit où ajouter (une section, un bloc).
2. Dans **Ajouter**, double-cliquez sur un élément, ou glissez-le sur la page.
3. Les **blocs prêts à l'emploi** (en-tête de page, avantages, questions fréquentes, formulaire…) ajoutent une section complète.

Pour déplacer un élément, glissez-le sur la page ou dans les **Calques**. Alt+glisser en fait une copie.

## Régler l'apparence

L'apparence se règle dans **Styles**, par **classe** : tous les éléments qui portent la même classe changent ensemble, comme dans Webflow.

1. Sélectionnez l'élément, puis tapez un nom de classe dans le champ en haut de **Styles** (par exemple `carte`).
2. Réglez la disposition, l'espacement, la taille, le texte, le fond, la bordure, les effets.
3. Un libellé **bleu** est réglé ici ; un libellé **orange** est hérité (d'un écran plus large ou de l'état normal). Cliquez sur un libellé pour voir d'où vient la valeur ou la retirer.

**Écrans** : choisissez la tablette ou le téléphone en haut ; ce que vous réglez alors ne vaut que pour cet écran et les plus petits.

**États** : le menu à côté de la classe règle le survol, l'appui ou le focus. « Ouvert (accordéon) » règle un accordéon ouvert : le canevas l'ouvre le temps du réglage.

**Maçonnerie** : le bouton de la disposition en grille range les éléments de hauteurs différentes en colonnes, sans trous ; « Colonnes à tour de rôle » les place dans l'ordre, une colonne après l'autre.

**Classes** : le panneau **Classes** liste les classes du site et les éléments qui les portent. **Renommer** change le nom partout (styles, éléments, actions) ; le filtre montre les classes inutilisées, que la corbeille retire. Double-cliquez sur une classe dans **Styles** pour la renommer.

**Couleurs et polices du site** : dans un sélecteur de couleur, **+** ajoute la couleur aux couleurs du site. **Paramètres › Général › Couleurs** les liste toutes ; **Mode sombre** donne à chacune une valeur pour les visiteurs dont l'appareil est en thème sombre, et le bouton lune de la barre d'outils montre le site ainsi. Les polices se gèrent dans **Paramètres › Général › Polices**.

## Textes

- **Double-cliquez** sur un texte pour l'écrire sur la page. **Entrée** valide, **Maj+Entrée** passe à la ligne, **Échap** annule.
- Sélectionnez des mots : une barre propose **gras**, *italique*, **lien** et **span** (une partie du texte à styler à part).
- Le champ **Texte** en haut de **Styles** modifie aussi le texte de l'élément choisi.

## Images et médias

1. Ajoutez une **Image**, puis **Choisir une image** dans **Styles**.
2. Importez vos fichiers dans **Médias** (glisser-déposer). Revel produit des images légères pour chaque écran.
3. Écrivez un **texte alternatif** : il décrit l'image aux personnes qui ne la voient pas.

Les **vidéos**, les **animations Lottie** (fichier `.json` ou `.lottie`) et les documents s'ajoutent de la même façon.

## Composants

Un composant est un morceau de page réutilisé partout (en-tête, pied de page, carte, service, outil). Clic droit sur un élément dans les calques › **Créer un composant**. Modifier le composant change toutes ses utilisations ; une utilisation peut avoir ses propres textes, images et liens (ses **propriétés**, dans **Styles**) et sa variante (survol, appui…).

**Emplacement** : dans le composant, l'interrupteur **Emplacement du contenu** d'un élément (la liste des outils d'une carte, les points d'un service) permet à chaque utilisation d'y mettre son propre contenu. Sur la page, **Modifier ici** copie le contenu du composant dans l'utilisation pour le changer à cet endroit seulement ; **Revenir au contenu du composant** l'annule.

Le CMS sert aux contenus qui ont leur propre page (projets, articles) ; les blocs d'une page, même répétés (services, outils, classements), sont des composants placés sur la page.

## Contenus (CMS)

1. Ouvrez **CMS** en haut. Créez une **collection** (Projets, Actualités…) et ses **champs** (titre, date, image, texte riche…).
2. Ajoutez des **contenus** : chacun est un fichier du site, modifiable aussi avec Pages CMS.
3. Dans l'éditeur, ajoutez une **Liste de collection** : un élément par contenu. Sélectionnez un texte ou une image de la liste et reliez-le à un champ dans **Styles › Relié au contenu**.
4. Chaque contenu a sa **page**, faite d'un modèle commun créé avec la collection : réglez-le une fois pour tous depuis **Pages**.

Une liste longue peut n'**afficher d'abord** que quelques contenus (réglage de la liste) : la suite apparaît quand le visiteur arrive en bas.

Une date reliée s'écrit comme vous le voulez (16 octobre 2026, 16.10.2026…) ; un élément peut n'être affiché que si un champ est rempli.

## Animations et actions

- **Interactions › Animations** : choisissez un moment (apparition, texte, survol, appui, boucle, défilement), puis une animation. Réglez la durée et le délai ; le bouton ▶ la rejoue.
- **Interactions › Actions** : un appui, un survol ou le défilement agit sur un élément (ouvrir un menu, rendre l'en-tête opaque, fermer un bandeau).
- **Paramètres › Général › Transition entre les pages** : fondu ou glissement d'une page à l'autre.
- **Paramètres › Général › Défilement doux** : la page, ou la colonne sous le pointeur, glisse vers sa destination à la molette.
- **Ajouter › Barre de progression** : se remplit avec le défilement de la colonne qui la contient, ou de la page (réglage « Suit »).
- **Suit la section affichée** (élément qui contient des liens vers des sections de la page) : le lien de la section en cours de lecture prend l'état « Actif ».
- Une action peut viser **l'élément d'une classe qui contient** celui qu'on appuie : chaque carte d'une liste s'ouvre seule.

## Langues

**Localisation** ajoute une langue au site. Les textes se traduisent à la main ou automatiquement, sur votre ordinateur (langue téléchargée dans **Réglages › Traduction automatique**). Une traduction automatique est marquée en violet tant qu'elle n'est pas relue.

## Publier

1. **Paramètres › Publication** : ajoutez un hébergement (OVH, o2switch, Netlify, GitHub Pages…). Revel guide chaque étape.
2. **Publier** en haut à droite envoie les pages modifiées. Une version de test peut précéder le site public.
3. **Aperçu** ouvre le site tel qu'il sera publié, animations et liens compris.

## Annuler et versions

- **Ctrl+Z** annule, **Ctrl+Maj+Z** rétablit.
- **Paramètres › Versions** garde l'historique du site : enregistrez une version, comparez, revenez en arrière.

## Travailler à plusieurs

- **Paramètres › Versions › Travail à plusieurs** : choisissez **Dossier partagé** (un dossier du réseau de l'équipe, NAS ou serveur : sans compte) ou **Dépôt en ligne** (GitHub, GitLab : utilisable partout). La fenêtre d'activation compare les deux.
- Chacun travaille sur sa copie du site puis clique **Synchroniser** : les versions des autres arrivent, les siennes partent. Les modifications faites à des endroits différents se combinent.
- Un même endroit modifié par deux personnes : choisissez **Ma version** ou **Version reçue** pour chacun, puis **Appliquer**.
- Les autres ouvrent le site avec **Importer › Dépôt Git**, en collant l'adresse copiée ou avec **Dossier…**. **Réglages › Comptes › Votre nom** inscrit le nom de chacun sur ses versions.
- Dépôt en ligne : chacun a son propre compte GitHub, invité au dépôt (Settings › Collaborators) ; GitHub interdit de partager un compte.

## Mises à jour de Revel

- À chaque lancement, Revel cherche une version plus récente ; un bouton **Version x.y.z** apparaît alors en haut à droite.
- **Réglages › Mises à jour** l'installe et relance Revel. Chaque version est vérifiée par sa signature avant d'être installée.
- **Recevoir les préversions** propose aussi les versions d'essai, publiées avant la version stable.

## Raccourcis utiles

| Action | Raccourci |
|---|---|
| Rechercher une commande | Ctrl+K |
| Annuler, rétablir | Ctrl+Z, Ctrl+Maj+Z |
| Dupliquer, supprimer | Ctrl+D, Suppr |
| Élément parent, enfant | Maj+Entrée, Entrée |
| Envelopper dans un bloc | Ctrl+G |
| Zoom ajusté, 100 % | Maj+1, Maj+0 |
| Masquer les panneaux | Ctrl+\ |

La liste complète est dans le menu **?** de la barre de l'éditeur.
