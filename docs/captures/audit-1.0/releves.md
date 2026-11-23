# Audit de Revel avant la 1.0 — relevés (23/11/2026, 0.8.1 + b97efa0)

Méthode : parcours de l'interface par Playwright (clics réels), captures dans audit/shots, inventaires dans audit/inv.
Format d'un relevé : [zone] constat — gravité (B bloquant, G gênant, M mineur) — type (incohérence, manque, doublon, en trop, panne).

## Relevés

### Accueil et réglages de Revel (inv/01–05)
- [Accueil › carte de site] 4 boutons d'icône par carte (Vignette, Paramètres, Retirer de la liste, Supprimer) ; pas de « Dupliquer le site », pas de « Renommer » direct, pas d'ouverture du dossier — M — manque (Framer : Dupliquer, Renommer, Déplacer dans le menu du projet). À VÉRIFIER dans Paramètres du site.
- [Accueil › Nouveau site] Création en un dialogue : Nom, Langue, Template (Vide / un au hasard « Vitrine » / Plus), Fond, Emplacement. La 2e tuile est un template tiré au hasard (classe template-choice--random) : imprévisible pour un débutant — M — incohérence.
- [Profil] Photo, Nom, GitHub présents à l'identique dans la bulle Profil ET dans Réglages › Profil et comptes — M — doublon (la bulle a aussi un lien « Réglages de Revel »).
- [Réglages › Éditeur] Sous le titre « Recherche rapide » se suivent les familles de la recherche (Actions… Aller à) PUIS des réglages sans rapport : Classes, Signaler les médias lourds, Montrer l'élément ajouté dans les calques, Ouvrir en canevas libre, Ouvrir avec tous les écrans — G — incohérence de rangement (à vérifier sur capture : sous-titre manquant ?).
- [Réglages › interrupteurs] L'interrupteur est dessiné comme un trait avec butée (ressemble à une glissière), pas comme une bascule — à vérifier en gros plan, deux thèmes.
- [Réglages › Éditeur › Variantes des composants] Forme des liens (Courbes/Équerre/Droits) et bout (Flèche/Rond) : réglage cosmétique de niche placé en tête de la section Éditeur — M — en trop ou à descendre.
- [Réglages › Raccourcis] 27 actions modifiables. Pas de raccourci listé pour : Aperçu, Enregistrer/Publier, Ajouter (texte T, cadre F, etc. chez Framer), bascule panneaux — à comparer.
- [Réglages › Templates] Catalogue par défaut « matv-studio/revel-templates » affiché « Introuvable » (simulateur ? à vérifier sur le vrai dépôt).

### Éditeur : première vue (inv/10, 11, 13)
- [Barre d'outils] Le bouton « + » du centre gère les points de rupture ; pour un utilisateur de Framer « + » = insérer un élément. L'insertion (« Ajouter ») est un onglet secondaire du dock bas-gauche, caché derrière « Calques » à l'ouverture — G — incohérence / découvrabilité (Framer : bouton Insert en tête de barre, touche I ? à vérifier).
- [Page vide] Le modèle « Vide » pose un H1 lié à l'information du site « Titre » et un paragraphe ; pas d'invite « Ajouter une section » sur le canevas — M — manque.
- [Fil d'Ariane bas] « Page › Section › Titre   Page index » : « Page index » sans libellé ni info-bulle lisible (nom de fichier ?) — M — incohérence.
- [Ajouter] 42 outils en 8 groupes. Rangement : « En-tête », « Pied », « Sélecteur de langue » sont dans le groupe « CMS » (ce ne sont pas des outils de CMS) ; « Lien » et « Bouton » dans « Texte » ; « Interrupteur » = bascule clair/sombre ou musique (nom trompeur : évoque un interrupteur de formulaire) — G — incohérence.
- [Ajouter] Absents à première vue (à confirmer en cherchant) : liste à puces, citation, séparateur/trait, espace, carte (Maps), intégration (embed YouTube hors Vidéo ?), fenêtre modale/overlay, info-bulle, recherche, pagination/« charger plus », curseur personnalisé, forme/SVG, tableau, champs de formulaire seuls, réseaux sociaux.
- [Styles] Modèle par classes (type Webflow) activé par défaut : champ « Classes », « Code CSS (avancé) », « Paramètres avancés » (balise, id, attributs, classes selon le contenu). Un utilisateur de Framer ne connaît pas les classes — décision à prendre : classes masquées par défaut (le réglage existe : Réglages › Éditeur › Classes).
- [Styles › Texte] 16 lignes dont 6 sur le retour à la ligne (Retour à la ligne, Coupure [text-wrap], Mots longs, Coupure [word-break], Tronquer, Lignes max) + Sens d'écriture + Contour + Ombre : deux lignes portent le même libellé « Coupure » — G — doublon de libellé ; trop de réglages de niche au premier niveau (à replier sous « Plus »).
- [Styles › Bordure] 4 lignes « Contour » (outline : épaisseur, style, couleur, écart) au premier niveau ; « Contour » existe aussi dans Texte (trait du texte) — M — libellé en double, niche.
- [Styles › Texte › Police] menu limité à system-ui / système / empattements / chasse fixe : aucune police Google ni lien « Ajouter une police » depuis le menu (les polices sont dans Paramètres › Général) — G — manque (Framer : recherche de police dans le champ).
- [Styles › menus] plusieurs menus listent deux fois la valeur courante (« Libres|Libres », « Normale|Normale », « Normal|Normal », « Automatique|Automatique ») — à vérifier à l'écran (option fantôme ?).
- [Titre] Bloc « Information du site » (Texte / Texte avant / Texte après) affiché en permanence sous le champ Texte de tout titre ou paragraphe ; deux champs nommés « Texte » l'un sous l'autre — G — incohérence (à replier derrière l'icône de liaison).
- [Effets] 14 lignes : Mélange, Opacité, Ombres, Transformations, Point d'appui, Perspective, Découpe, Masque, Transitions, Filtres, Filtres d'arrière-plan, Curseur, Clics, Empilement — Curseur/Clics/Empilement ne sont pas des « effets » — M — rangement.

### Outils de structure (inv/tool-insert-section|container|grid|columns)
- [Conteneur] L'outil s'appelle « Conteneur », le calque créé « Bloc », le raccourci « Envelopper dans un conteneur », l'affichage « Bloc » — M — incohérence de nom.
- [Grille / Colonnes / Conteneur] Trois outils posent la même boîte avec un affichage différent (Bloc, Grille 2 col., Flex à la ligne 2 blocs) ; le réglage « Affichage : Bloc / Flex / Grille / Masqué » permet de passer de l'un à l'autre. « Grille » (2 colonnes) et « Colonnes » (2 colonnes) donnent le même rendu à l'insertion — G — doublon apparent pour un débutant (Framer : Stack et Grid, différence visible : direction/écart vs colonnes fixes).
- [Disposition] Libellé « Flex » (jargon CSS) ; Framer dit « Stack ». « Masqué » est rangé comme un mode d'affichage alors que le panneau a déjà l'œil « Masquer le calque » en tête — G — doublon de réglage (2 façons de masquer : œil du calque, Affichage › Masqué ; + « Visibilité » ? à vérifier).
- [Structure] Aucun réglage propre en tête de panneau (normal). La boîte d'alignement 3×3 + menus X/Y + « Plus d'options d'alignement » (Colonnes/Lignes) : 3 niveaux pour l'alignement en grille — M.

### Outils interactifs (inv/tool-insert-navbar … pixels)
- [Barre de navigation] posée dans la page (main), pas dans la zone « En-tête » du site ; rien ne propose de la mettre en en-tête commun — G — à comparer (Framer : composant posé sur chaque page ; Webflow : symbole). Réglages : Marque, Logo, Menu replié (point de rupture), icône du bouton, « Montrer le menu », Liens.
- [Carrousel] « Proportions » : liste propre (Libres|1:1|4:3|3:2|16:9|3:4) différente de celle de Styles › Taille › Proportions (Carré 1:1, Large 16:9, Classique 4:3, Paysage 3:2, Portrait 2:3, Vertical 9:16) — M — incohérence. « Arrondi » et « Écart » redoublent Styles — M. Pas de réglage de transition (fondu `fade`, `hold` : attributs écrits à la main, D-190) — G — manque connu. Hauteur à l'insertion : 800 px de cases vides.
- [Bandeau cookies] menu « Consentement aux cookies : Aucun rôle | Bandeau | Bouton Accepter | Bouton Refuser | Lien Gérer » exposé sur le bandeau lui-même (mécanique interne) ; identifiant Google Analytics réglé sur le bandeau (à croiser avec Paramètres du site : doublon ?) — G.
- [Interrupteur] « Commande : Thème / Musique / Sons » ; « Texte quand allumé / éteint » ET bloc « Textes : Libellé, État » (État = « Sombre » = texte quand allumé) — G — doublon de réglage. Musique/Sons : fonctions de niche (import p3r) au même rang que le thème.
- [Onglets] « Au survol », « Touches du clavier (flèches, W A S D) », « Son au changement » : réglages de niche (menu de jeu) en tête de l'outil — M — en trop ou à replier.
- [Sommaire] « Zone » liste des noms de classes bruts (.intro, .page, .section, .titre) — G — incohérence (jargon ; devrait lister les calques par leur nom).
- [Horloge], [Toile de pixels] outils de niche au même rang que Barre de navigation / Carrousel dans « Interactifs » — M — en trop au premier niveau (groupe « Plus » ?).
- [Contrôles] Les cases oui/non sont tantôt des cases à cocher (« Flèches », « Compteur », « Boucle », « Secondes »), tantôt des interrupteurs (« Pause au survol », « Tronquer ») dans les mêmes panneaux — G — incohérence de contrôle.
- [Filtres de liste] colonnes « Nom » / « Valeur » et « Paramètre d'adresse » : vocabulaire technique ; lien avec la liste filtrée non montré dans l'outil (à tester avec une liste).

### Outils texte et médias (inv/tool-insert-heading … model)
- [Vidéo] deux champs pour la même chose : « Vidéo (adresse) » en tête et « Source » sous « Lecture » (capture tool-insert-video) ; l'icône du premier est une icône d'image — G — doublon. « Lecture automatique (sans le son) » et « Sans le son » : deux cases liées — M. « Fonction : Survol » et bloc « Média au survol › Change avec l'élément survolé » : même fonction à deux endroits — G — doublon.
- [Image] champ « Image (adresse) » (texte) + icône « Choisir parmi les médias » : pas de bouton « Importer / déposer une image » dans le panneau ; une image vide mesure 160×100 — G — à comparer (Framer : clic sur le remplissage → choix/dépôt du fichier). « Fonction : Image / Révélation » (seconde image sous le pointeur) : niche en tête de panneau — M.
- [Lien] [Bouton] mêmes réglages (Texte, Adresse : Page|Section|Fichier|Adresse|E-mail|Téléphone, nouvel onglet, « Actif sur les sous-pages »). Posés à la suite, Lien, Bouton et Image se suivent sur la même ligne sans écart (éléments en ligne) — M.
- [Icône] 7 familles en onglets + « Importées » ; noms d'icônes en info-bulle sans accents (« etoile », « trophee ») ; pas de champ de recherche d'icône vu — M — manque (à confirmer).
- [Modèle 3D] ~30 réglages (Caméra, Rendu, Boutons) ; « Rotation automatique » figure deux fois (vitesse sous Caméra, bouton sous Boutons) — M — libellé en double.
- [Interrupteurs] dessin : trait court gris (éteint) / trait long coloré avec butée (allumé) — faible lisibilité pour un débutant (captures tool-insert-video) — G — à revoir (bascule classique).
- [Bandeau cookies] s'affiche fixe par-dessus le canevas et masque la zone de travail en bas à droite pendant l'édition — G — à vérifier s'il existe un moyen de le masquer (œil du calque).

### Blocs, CMS, formulaires, code (inv/tool-insert-block-* … embed)
- [Blocs] « En-tête de page » (bloc d'accroche / hero) et « En-tête » (zone commune du site) : deux outils au nom presque identique pour deux choses différentes — G — incohérence (Framer : « Hero » / « Navigation »).
- [Blocs] pastille « Nouveau » en tête de Styles après la pose d'un bloc (sens donné seulement en info-bulle) — M.
- [Champ CMS] posé hors d'une liste ou d'un modèle de pages : un paragraphe ordinaire est créé, sans message ni réglage de champ — G — outil sans effet visible hors contexte (à désactiver ou à guider).
- [Liste de collection] sans collection : menu « Collection » = « all » (valeur brute, non traduite) + second menu « + Autre collection » ; rien n'invite à créer une collection — G — incohérence / manque. 9 réglages à plat (Contenus Auto/Manuel, mis en avant, Seulement si le champ, Filtre, Tri, Ordre, Nombre, À partir du n°, Nombre avant « Voir plus ») — M — à grouper.
- [Bouton « Ajouter au panier »] (et sans doute tout élément <button>) porte deux menus de rôle : « Consentement aux cookies : Aucun rôle | Bandeau | Accepter | Refuser | Gérer » et « Interrupteur : Aucun | Musique | Sons | Thème » — G — mécanique interne exposée ; doublon avec les outils « Bandeau cookies » et « Interrupteur ».
- [Formulaire] réglages du bloc : « Envoyé par le site », « Nom du formulaire », lien vers Paramètres › Formulaires. Champs : à relever dans le site d'essai.
- [Panier] seul réglage : Monnaie (10 codes). [Code HTML] zone de texte.
- [Zones En-tête / Pied] pastille « Tout le site », case « Montré sur cette page ».

### Vues CMS, Localisation, Plan du site, Paramètres › Général (inv/30-vue-*)
- [CMS vide] « Aucune collection dans ce site. » + une petite icône « Nouvelle collection » : pas de bouton principal ni de modèles de collection (Blog, Projets…) — M — manque (Framer : « Add collection » avec modèles).
- [Paramètres › Général] le nom du site se règle à trois endroits : Identité › Nom, Informations du site › Titre, Référencement › Nom affiché du site ; la description à deux (Informations du site › Description, Référencement › Description par défaut) ; la langue à deux (Identité › Langue, Informations du site › Langue = « fr ») — G — doublons de réglages.
- [Paramètres] trois dessins de case oui/non dans la même vue : case à cocher (« Typographie française », « Défilement doux »), interrupteur en ligne (« Mode sombre »), ligne à case (« Plan du site ») — G — incohérence de contrôle.
- [Paramètres › Couleurs] les blocs posés ajoutent des couleurs nommées « bloc-accent », « bloc-accent-texte » (noms techniques avec tirets, champ en chasse fixe) — M.
- [Paramètres › Formulaires] « Destinataire selon le champ » : nom de champ à taper à la main ; lien « Recette OVH » propre à un hébergeur — M.
- [Paramètres › Référencement › Titre de l'onglet] bien fait (segmenté + exemple).
- [Localisation] langues proposées (13 + Autre) ≠ langues de l'interface de Revel ; tableau Langue/Code/Adresse/Traduction/Publiée.
- [Plan du site] filtres Parties communes / Autres sites / Médias et réseaux, « 1 adresse sans page » (liens À propos et Contact de la barre de navigation posée) : utile.

### Paramètres › Publication, Navigation, Sauvegarde (inv/31-param-*)
- [Publication] contient aussi « Template » (enregistrer le site comme template) et « Code personnalisé » (</head>, </body>) : deux rubriques sans rapport avec la publication — G — rangement (Framer : Custom code dans General).
- [Publication] deux boutons « Publier » : barre du haut et carte de l'hébergement ; à tester : font-ils la même chose ?
- [Navigation › Menus du site] « listes de liens reprises par les menus des pages » : à croiser avec la liste « Liens » de l'outil Barre de navigation — doublon possible (deux manières de gérer les liens du menu).
- [Sauvegarde] deux boutons Local / Dépôt en ligne (Git) ; vocabulaire « dépôt », « Git » pour un débutant — M.

### Interactions, Classes (inv/32-*, 33-*)
- [Interactions] 9 familles : Apparition (13 modèles), Texte (7), Survol (9), Appui (3), Boucle, Défilement, Devenu actif, Glisser, Actions. Réglages : Déclencheur, Durée, Délai, Courbe (Douce|Rebond|Régulière|Linéaire), Décalage, Rejouer / Intensité.
- [Interactions] PANNE i18n : info-bulles « anim-d-appear-unveil » (modèle « Dévoiler une image ») et « anim-d-hover-follow » (modèle « Suivre ») = clés de traduction non traduites — G — panne.
- [Interactions › Survol / Appui] vs [Styles › État : Normal → Survol…] vs [Styles › Effets › Transitions] : trois endroits pour un effet au survol — G — doublon à arbitrer (Framer : un seul, les effets ; Webflow : états + interactions).
- [Interactions] « Courbe » : 4 choix sans courbe personnalisée ni ressort (Framer : spring, bezier) — M — manque à confirmer.
- [Classes] après la pose des blocs : 39 classes aux noms techniques (bloc__inner, bloc-bouton--secondaire, panier__moins…) ; pied : « Seulement les inutilisées », « Regrouper les classes de mêmes valeurs », « Rendre réglable dans le panneau Styles (10 réglages écrits en code) » — les blocs fournis par Revel contiennent donc 10 réglages que le panneau Styles ne sait pas régler — G — incohérence (blocs natifs non entièrement réglables).
- [Interactions] **PANNE BLOQUANTE (reproduite, a34-crash.mjs)** : après un ajout, un changement ou un retrait d'animation, désélectionner (Échap, clic ailleurs, autre élément) dans les 350 ms fait planter toute l'interface (« panicked … interactions.rs:1006 … reactive value … already disposed », puis `RuntimeError: unreachable` : plus aucun bouton ne répond). Cause : `set_timeout(preview, 350 ms)` (interactions.rs:1025, aussi 967) lit un signal du panneau déjà détruit. — B — panne. Correction simple (lecture `try_get_untracked`).
- [Interactions › Boucle] 9 modèles dont « Bandeau » (contenu qui défile sans fin) = l'outil « Défilement » du volet Ajouter — G — doublon d'outil.
- [Interactions] précision sur la panne : fenêtre de 350 ms après chaque écriture ; reproduite aussi sans Échap dans une suite rapide (ajout Défilement, relevé, retrait). Un retrait lent ne plante pas.
- [Interactions › Défilement] 7 modèles (Parallaxe, Fondu, Zoom, Rotation, Glissement, Sortie, Gouttes), seul réglage : Intensité. Info-bulle « anim-d-scroll-drops » non traduite (3e clé manquante). Pas de réglage de plage de défilement (début/fin), ni de cible — M — manque vs Framer (scroll transform avec bornes).
- [Interactions › Devenu actif] Rebond, Saut, Secousse, Éclair, Rotation. [Glisser] aucun réglage (ni axe, ni limites) — M — manque (Framer : direction, contraintes, élan).
- [Interactions › Actions] déclencheurs : À l'appui | Au survol | Au défilement | À l'écran | Au chargement ; actions : Ajouter une classe | Afficher ou masquer | Afficher | Masquer | Lire ou mettre en pause | Fermer pour de bon | Afficher la suite d'une liste | Copier l'adresse | Partager la page | Un enfant au hasard | Tourner | Jouer un son | Lancer une musique. Action par défaut « Ajouter une classe », cible choisie par « Classe « bloc__inner » »… : entièrement fondé sur les classes — G — à revoir en mode sans classes (cible = calque nommé). Pas d'action « Ouvrir une fenêtre / un volet (overlay) », « Aller à une section », « Changer de variante » — G — manque vs Framer (overlays, variantes).

## MODE SANS CLASSES (demande du 23/11 16:31) : paramètre `simple=1` du simulateur = Réglages › Éditeur › Classes désactivé. Tout ce qui suit est relevé dans ce mode.

### Mode sans classes : premiers gestes (b01)
- [Styles sans classes] le champ Classes, l'onglet Classes et le bouton d'état (Normal / Survol…) disparaissent : il ne reste aucun moyen de régler l'aspect d'un élément au survol (couleur d'un bouton, soulignement d'un lien) hors des 9 modèles d'Interactions › Survol — G — manque (Framer : effet Hover libre ou variante de survol). À confirmer dans le site d'essai.
- [Styles sans classes] restent visibles : info-bulle « .titre — Ordinateur (base) » (nom de classe), section « Code CSS (avancé) », « Paramètres avancés › Classes selon le contenu » (« Classe ajoutée », « Classe selon une condition ») — G — incohérence avec le mode.
- [Titre] « Niveau H1…H6 » en tête ET « Paramètres avancés › Balise : Titre 1…6 » — M — doublon de réglage.
- [Texte sur la page] double-clic : n'édite que si le pointeur est sur les lettres ; double-clic dans la boîte du texte à côté des lettres : rien (Framer : double-clic n'importe où dans la boîte) — M.
- [Texte sur la page] barre flottante B, I, lien, « [ ] » (icône sans sens évident) ; elle recouvre le titre du dessus et l'étiquette du calque ; Ctrl+B / Ctrl+I au clavier sans effet — M — manque.
- [Titre lié à l'information du site] modifier le H1 de la page d'accueil change le « Titre » du site partout (4 usages) sans le signaler au moment de la saisie — G — incohérence pour un débutant (le modèle Vide part d'un titre lié).
- [Menu contextuel] Sélectionner le parent, Zoom, Copier, Couper, Coller, Dupliquer, Envelopper, Retirer l'enveloppe, Monter, Descendre, Créer un composant, Supprimer. Absents : Renommer, Masquer, Copier/Coller le style, Ajouter (insérer ici), Verrouiller — M — manque vs Framer.
- [Ctrl+K] familles Actions, Affichage, Ajouter (tous les outils), Pages… : complet.

### Pages et paramètres de page (b03, b04)
- [Pages] création en ligne (nom tapé dans la liste), adresse déduite, en-tête du site repris, section + H1 au nom de la page : bien. Structure différente de l'accueil du modèle Vide (accueil : Page › Section ; nouvelle page : Section seule) — M — incohérence.
- [Zone En-tête] créée avec un paragraphe d'attente « En-tête » à supprimer à la main — M.
- [Paramètres de page, mode sans classes] « Classe du corps de page » (champ en chasse fixe), « Type au partage (og:type) », « Propriétés du gabarit », « Date de publication » sur toute page : réglages techniques au même rang que Nom / Adresse — G — à replier (« Avancé ») ; le champ de classe contredit le mode sans classes.
- [Paramètres de page] bien : aperçu du résultat de recherche, « Rediriger l'ancienne adresse », Brouillon, Dupliquer / Supprimer.

### Barre de navigation, blocs (b05, b06)
- [Barre de navigation › Liens] les 3 liens d'origine pointent tous vers « / » ; créer ensuite les pages « À propos » et « Contact » ne les relie pas — M. « Ajouter un lien » duplique le dernier lien (texte et adresse « Contact ») au lieu de créer un lien vide à nommer — G — incohérence. Pas de renommage dans la liste (il faut cliquer la ligne, ce qui quitte la barre et sélectionne le lien, puis revenir) alors que Onglets / Accordéon / Filtres se renomment dans leur liste — G — incohérence entre deux modèles de liste (items-list / parts-list).
- [Barre de navigation › Menu replié] propose « Petit téléphone (479 px) » alors que le site n'a que les points de rupture 991 et 767 — M — incohérence.
- [Blocs] un bloc « section complète » posé pendant qu'une section est sélectionnée se place DANS cette section (section dans section) et non après elle — G — incohérence (Framer : une section s'ajoute à la suite dans la page).
- [Blocs › noms de calques] « Bloc inner », « Bloc titre », « Bloc actions », « Bloc bouton secondaire » : noms tirés des classes (dont « inner » en anglais), visibles même en mode sans classes — G — incohérence.
- [Vocabulaire] même objet, plusieurs noms : calque « Pied » / fil d'Ariane et panneau « Pied de page » ; canevas « Bouton » / panneau « Interrupteur » ; outil « Accordéon » (réglage « Volets ») / calques du bloc Questions « Dépliant », « Titre du dépliant » ; « Liste · Locales » (terme anglais) sous Sélecteur de langue — G — incohérence.
- [Fil d'Ariane bas] « Gabarit base » / « Page index » affichés à droite du fil sans libellé — M.
- [Couper-coller de calque] Ctrl+X puis Ctrl+V sur « Page » : fonctionne (bloc sorti de la section).

### Site généré après composition de l'accueil (shots/site-accueil.png)
- [Blocs vs outils] les blocs (Accroche, Avantages, Texte et image, Témoignage, Questions, Appel à l'action) ont marges et largeur de contenu ; les outils posés au même niveau (Carrousel, Défilement, Onglets) collent aux bords de la page, sans marge ni largeur maximale — G — incohérence : il faut les envelopper à la main dans une section.
- [Carrousel vide] sur le site généré : compteur « 1 / 0 » et flèches seuls ; [Défilement vide] : rien ; [Sélecteur de langue] invisible avec une seule langue (aucune indication dans l'éditeur) — M.
- [Zones] le pied garde son paragraphe d'attente « Pied » sur le site publié si on ne le retire pas — M.

### Médias (b08, b09)
- [Médias] import par le bouton du pied de panneau : fonctionne (10 fichiers). Pas de zone « glisser des fichiers ici » visible dans le panneau vide — M.
- [Médias › menu contextuel] Détails | Placer dans la page | Copier l'adresse. Le clic droit ouvre aussi la fiche « Détails » sous le menu ; Échap ferme le menu mais pas la fiche (retour par la flèche seulement) — M — incohérence.
- [Médias › fiche] Adresse, Taille, Texte alternatif par défaut, Remplacer, Placer. Absents : Supprimer, Renommer, Déplacer dans un dossier — G — manque (à confirmer : pied du panneau a « Nouveau dossier »).

### Carrousel, choix d'image (b09, b10)
- [Choix d'image] fenêtre « Choisir une image » (recherche, vignettes, import) : bien.
- [Carrousel] le compteur du canevas passe de « 1 / 3 » à « 1 / 0 » après le choix des images (capture b10-carrousel) ; site généré avec carrousel vide : « 1 / 0 » — G — panne d'affichage à vérifier sur le site généré avec images.
- [Carrousel › élément] panneau intitulé « Élément 1 », étiquette du canevas « Image » puis « Bloc » pour le carrousel lui-même (panneau « Carrousel ») — incohérence d'étiquettes (déjà relevée). Dans « Fonction : Image | Révélation », le mot « Révélation » est recouvert par l'icône de liaison (capture b09-carrousel-element) — M — défaut d'affichage. Segmenté « Images Contenu Mosaïque » : libellés serrés.
- [Styles] section « Enfant flex » (« Modifier le flex parent », Taille « Ni grandir ni rétrécir », Aligner, Ordre) : jargon CSS — G — à renommer (« Dans son conteneur »).
- [Zoom] le menu affiche « 84 % » au lieu de « Ajusté (84 %) » après une sélection par les calques — M.
- [Calques] le carrousel ne se déplie pas dans les calques (ses diapositives n'y figurent pas) alors que le fil d'Ariane montre Carrousel › Bloc › Image — M — incohérence.
- [Défilement] « Ajouter une image » ajoute après les 4 éléments d'attente (« Élément 1…4 ») qui restent : 8 éléments, les vides à retirer un par un — M.
- [Image] réglages différents selon l'origine : image posée par l'outil = « Fonction : Image | Révélation » ; image d'un bloc = pas de Fonction mais « Média au survol › Change avec l'élément survolé » ; recadrage (Zoom, X, Y) seulement une fois l'image choisie — M — incohérence. « Média au survol » sur toute image : niche en tête de panneau.

### Styles en mode sans classes (b12, b13)
- [Nuancier] pipette, modèle (Hex…), opacité, « Couleurs du site » + ajout : bien. Pastilles des couleurs du site sans nom visible (info-bulle seulement).
- [Styles partagés] en mode sans classes, régler « Bloc sous titre » d'un bloc règle les 4 sous-titres de la page (même classe `.bloc-sous-titre`) ; le panneau l'indique par « Ordinateur · 4 éléments », sans dire lesquels ni offrir « seulement celui-ci » — G — incohérence majeure du mode sans classes : l'utilisateur ne voit pas les classes mais en subit le partage. (Framer : un style appartient au calque ; le partage passe par les styles de texte / composants, explicitement.)
- [Taille de texte] pastille « Fluide 26px → 40px » ; bulle Min. / Idéal (« 3.5vw ») / Max. / « Valeur fixe » : l'unité vw est du jargon — M.
- [Points de rupture] réglage à ≤ 767 px écrit dans une règle média, ordinateur inchangé : fonctionne. En-tête du panneau : « ≤ 767 px · 1 élément ».
- [Étiquettes du canevas] « Bloc hero », « Bloc sous titre » (noms de classes) ≠ calques « En-tête de page » — déjà relevé.

### Composants (b14–b17)
- [Création] sélection → Composants › « + » → dialogue de nom : simple. Seule la carte sélectionnée devient une utilisation ; les 2 cartes identiques voisines restent des éléments libres (pas de proposition « remplacer les éléments semblables ») — M.
- [Écran du composant] variante « Principale », « + » pour une variante, barre basse « ← Accueil | Carte avantage | Sans transition » : proche de Framer. Le composant est montré sur toute la largeur du cadre (1216 px) et non à sa taille dans la page — M.
- [Relier] bouton « Relier à une propriété » → « Nouvelle propriété « Texte » » en un clic : bien. Mais la rubrique « Relié aux propriétés » est séparée du champ (le champ Texte n'a pas son bouton de liaison à côté ; l'icône à droite de « Niveau » est celle des informations du site) — G — incohérence de placement. La bulle recouvre les lignes Texte / Visibilité.
- [Propriétés] nommées « Texte », « Texte 2 » (et non d'après le calque : « Titre », « Paragraphe ») — M. Types : Texte, Image, Lien, Couleur, Visibilité, Liste. Absents : choix parmi des valeurs (énumération), nombre, icône, variante — G — manque vs Framer.
- [Icône dans un composant] une icône ne peut être reliée qu'à « Visibilité » : impossible de changer l'icône par utilisation (les 3 cartes « Avantages » ont 3 icônes) — G — manque.
- [« Emplacement du contenu »] interrupteur présent sur chaque élément du composant (titre, icône, racine) : notion de « slot » montrée partout — M — à replier.
- [Variantes] « + » → Survol | Appui | Au clic… | Variante personnalisée… ; variante de survol reliée par un trait « Au survol », réglages faits dans la variante écrits en `:hover` : très proche de Framer, fonctionne. C'est donc la voie du survol en mode sans classes (il faut d'abord faire un composant) — à dire quelque part : rien ne l'indique depuis un élément ordinaire (un bouton, un lien).
- [Variantes × classes] le survol du composant est écrit sur `.bloc-avantage:hover` : les 2 cartes voisines restées libres portent la même classe et prennent donc aussi le survol — G — fuite de style (conséquence du partage par classes en mode sans classes).
- [Utilisation] panneau : « Propriétés » (Titre, Description, valeurs par défaut en grisé), boutons Modifier le composant / Détacher : clair.
- [Panneau Composants] double-clic = ouvrir le composant (dans « Ajouter », double-clic = poser) — M — incohérence de geste. Menu : Modifier | Placer dans la page | Renommer | Supprimer ; pas de « Dupliquer » — M. « Placer dans la page » et glisser vers le canevas : fonctionnent (le glisser dépose dans le conteneur sous le pointeur).

### CMS (c01–c05)
- [Nouvelle collection] nom seul ; l'écran s'ouvre sur « Champs » (schéma) avec Titre et Adresse de la page. Chemin « content/projets » affiché à côté du titre ; info-bulle « schéma partagé avec Pages CMS » ; « .pages.yml » en pied de champ ; « Nom dans les fichiers » dans chaque champ — M — jargon technique visible.
- [Types de champ] Texte, Texte long, Texte riche, Image, Fichier, Date, Oui / non, Nombre, Choix, Référence, Code, Groupe de champs, Blocs (13). Absents vs Framer : Couleur, Lien, Galerie (plusieurs images), Vidéo — G — manque à confirmer. « Code », « Groupe de champs », « Blocs » : avancés, au même rang.
- [Nouveau champ] menu de type → dialogue « Libellé » → il faut rouvrir le champ pour ses réglages (choix d'un champ Choix, obligatoire…) : deux temps — M.
- [Fiche d'un contenu] deux éditeurs nommés « Texte » quand on crée un champ Texte riche appelé « Texte » : le corps du contenu (Markdown, absent de la liste des champs) porte déjà ce nom — G — incohérence : le corps intégré n'apparaît pas dans « Champs », rien n'empêche le doublon de nom. L'un édite du HTML, l'autre du Markdown (« Voir et modifier le HTML / le Markdown »).
- [Texte riche] barre : type de paragraphe (Paragraphe, Titre 2-4, Citation), Gras, Italique, Lien, listes, retirer la mise en forme, source. Pas d'image, de vidéo, de tableau ni de code dans le texte — G — manque vs Framer.
- [Tableau des contenus] tri, largeur des colonnes, choix des colonnes, « épingle » par champ dans la fiche : bien.
- [Modèle des pages] créé d'office (titre + contenu Markdown, en-tête et pied repris) ; bouton « Revenir à Projets » : bien. L'onglet CMS reste actif alors qu'on est dans l'éditeur. Dans Pages : ligne « Projets » (page) et groupe « Projets 5 » (collection) : deux « Projets ».
- [Champ CMS dans le modèle] réglages : Collection, Affiché en (Texte | Image | Lien), Champ, Sinon, Texte si vide, Mettre en valeur les *mots*, Texte avant / après, Si le champ est vide (Garder | Masquer | Selon une condition) : complet et lisible.
  - deux rubriques l'une sur l'autre : « Champs CMS (1) » (valeur lue, lecture seule, « Modifier ce contenu dans le CMS ») et « Champ CMS » (réglage) — G — libellés quasi identiques.
  - le calque posé par l'outil s'appelle « Paragraphe » (celui du modèle d'origine « Champ CMS ») — M.
  - menu « Champ » : deux entrées « Temps de lecture de « … » tronquées et indiscernables — M.
- [Champ CMS en image] champ « Image (expression) » en chasse fixe, valeur « image » : mécanique interne montrée ; « Texte alternatif » non reliable à un champ depuis ce panneau — M.
- [Calques] le champ posé s'appelle « Paragraphe » jusqu'au rechargement, puis « Champ CMS · Projets › Résumé » — M — incohérence.

### Liste de collection, carte et composant (c10–c12) — au cœur de la refonte D-191
- [Liste de collection] pose d'office un Article (Titre + lien « Voir ») relié à la collection : bien. Calques : « Liste de collection » › « Liste · Projets » › Article (deux niveaux nommés « Liste »).
- [Liste › Tri] le menu liste « Date » deux fois (Ordre de la collection | Date | Titre | … | Date | …) — M — doublon d'option. « Seulement si le champ » et « Filtre » : deux filtres aux libellés proches — G — à fusionner / renommer.
- [Lien « Voir » de la carte] l'adresse est la page du contenu (`item.url`, liens générés corrects) mais le panneau montre « Page : Choisir une page… » — G — incohérence (devrait dire « Page du contenu »).
- **[Composant créé depuis une carte de liste]** « Créer un composant » sur l'Article : le composant garde ses lectures directes (`item.data.image`, `item.data.title`, `item.url`) ; aucune propriété n'est créée ; l'utilisation n'a ni rubrique « Contenu » ni propriétés. Posé hors d'une liste (« Placer dans la page »), il n'offre ni Collection ni Contenu à choisir : la carte n'est pas pilotable de l'extérieur — **B** — c'est le cas d'usage de la refonte D-191 (carte de projet choisie) ; la conversion D-191 ne couvre que les anciens composants à propriété « Contenu », pas la création d'un composant à partir d'une carte de liste.
- [Sélection du parent] Maj+Entrée juste après un clic dans les calques n'a pas sélectionné le parent (le clavier reste dans le panneau ?) — M — à vérifier.
- [Filtres de liste × liste du CMS] l'outil crée « Tout / Option 1 / Option 2 » avec des « Valeurs » à taper ; rien ne propose de créer les boutons d'après un champ (les 3 catégories de la collection) ni de désigner la liste filtrée ; la façon de marquer les cartes n'apparaît nulle part dans le panneau — G — manque (Framer : filtre de collection réglé sur la liste, pas de boutons-filtres natifs ; Webflow : idem via Finsweet) : à cadrer, c'est un atout si c'est guidé.
- [Calques de liste] « Liste de collection » (enveloppe) et « Liste · Projets » montrent exactement le même panneau de réglages — M — doublon de calque.

### Formulaire (d01, d02)
- [Champ du formulaire] Type (Texte, E-mail, Téléphone, Adresse web, Nombre, Date, Case à cocher, Choix unique, Fichier joint, Mot de passe, Caché), Nom envoyé, Obligatoire, Texte d'exemple, Saisie automatique : complet et clair. Absent : liste déroulante (select) — G — manque. La zone de texte longue n'a pas de « Type » (élément à part).
- [Ajouter un champ] aucun outil « Champ de formulaire » dans Ajouter (recherche « champ » → « Champ CMS », « Ajouter au panier » ; « case » → rien) : il faut dupliquer un champ existant — G — manque (Framer : Input, Textarea, Select, Checkbox, Radio dans Insert › Forms).
- [Formulaire] message après l'envoi et destinataire réglés pour tout le site (Paramètres › Formulaires), pas par formulaire ; pas de redirection après envoi — M — manque.
- [Calques] deux calques « Formulaire » emboîtés (section du bloc et formulaire), « Champ libelle », « Champ saisie », « Formulaire etat », « Formulaire ligne » : noms de classes sans accents — G (déjà relevé pour les blocs).

### Localisation (d03)
- [Ajout d'une langue] un clic, adresse /en/, tableau source / traduction par page, filtres « à traduire », « à relire », traduction automatique hors ligne (désactivée tant que la langue n'est pas téléchargée, info-bulle explicite) : bien fait, au-dessus de Framer (payant chez Framer).
- [Onglets Pages | Données | Contenus] « Données : textes des fichiers de données » — M — jargon.
- [Textes à traduire] un texte « Champ CMS » figure parmi les textes de l'accueil (texte d'attente de l'outil Champ CMS resté dans la page) — M.

### Barre du haut, aperçu, publication (d04, d05)
- [Menu du site] Récents, Tous les sites, Réglages de Revel : ni Dupliquer, ni Renommer, ni Ouvrir le dossier, ni Exporter — M — manque.
- [Aperçu] dans l'espace de travail (ordinateur / tablette / téléphone, recharger, nouvelle fenêtre, Fermer) : bien. L'onglet « Éditeur » reste actif pendant l'aperçu.
- [Publier] bulle : hébergement, « Jamais publié », « Hébergements », « Publier ». Sans hébergement réglé : à vérifier (parcours de première publication d'un débutant : Framer publie en un clic sur un sous-domaine gratuit ; Revel demande un hébergement SFTP / GitHub Pages) — G — à cadrer pour la 1.0 (assistant de première publication).
- [Champ CMS hors liste] confirmé à l'aperçu : la carte « Précis » affiche le texte « Champ CMS » (paragraphe littéral posé par l'outil hors contexte).

### Panneaux Variables et CMS de l'éditeur (d06)
- [Panneau Variables] « Aucune variable sur cette page », aucun bouton pour en créer ni explication : ce panneau montre les variables de données d'une page (sites importés), pas les « variables » au sens de Framer / Webflow (couleurs, tailles : ici dans Paramètres › Couleurs / Tailles) — G — nom trompeur ; panneau sans usage dans un site construit dans Revel (masqué par défaut : candidat au retrait de l'interface standard).
- [Panneau CMS] la collection y est nommée « projets_2 » (identifiant interne, suffixé parce qu'une page « projets » existe) au lieu de « Projets » — G — incohérence.

### Site généré final (shots/site-accueil.png, site-projet, site-contact, site-accueil-tel, site-en)
- Génération : 22 fichiers, 57 images produites, 2,3 s ; avertissements clairs (« plan du site non produit : adresse du site absente », « formulaires non reçus : adresse de réception absente »).
- [Survol] confirmé sur le site généré : les 3 cartes « Avantages » prennent le fond de survol alors qu'une seule est une utilisation du composant.
- [Carrousel] compteur correct sur le site (« 1 / 3 ») : le « 1 / 0 » n'existe que dans le canevas de l'éditeur.
- [Champ Choix] la liste affiche la valeur enregistrée (« site-web », « identite », « objet ») et non le libellé du choix (« Site web », « Identité ») — G — incohérence.
- [Filtres posés dans la liste] le groupe de filtres devient une case de la grille (« Tout » en pilule étirée sur la hauteur) ; les boutons sans élément correspondant sont masqués (D-190) : il ne reste que « Tout » — G — résultat incompréhensible sans guidage.
- [Outils hors blocs] liste, carrousel, défilement, onglets : pleine largeur, sans marge ni écart ; les blocs seuls donnent un rendu soigné.

## Framer : points vérifiés sur le web (23/11/2026)
- Formulaires : 4 familles de champ (texte, case, boutons radio, liste déroulante), ajoutées par le « + » de la barre du canevas quand le formulaire est sélectionné ; types de texte : texte, zone de texte, e-mail, nombre, téléphone, URL, date, heure ; après l'envoi : redirection, overlay, ou état du bouton / du formulaire ; protection anti-spam intégrée. (framer.com/updates/forms, help « Add different field types to a native form », academy « Creating your first form »)
- Effets : Appear, Hover, Press, Loop, Drag, Scroll Animation / Transform / Speed / Variant ; Hover et Press s'appliquent à un calque seul, sans composant, et comprennent remplissage et ombre. (framer.com/updates/all-new-effects, academy)
- Composants : variantes, états hover / pressed, variables d'instance (texte, liens, couleurs, images, visibilité, icônes), contrôle de variante sur l'utilisation. (academy « Component variants », « Component variables »)
- CMS : champs booléen, couleur, lien, date, option (enum), référence et multi-référence, etc. (framer.com/developers/cms)
