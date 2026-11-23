# Audit de l'interface avant la 1.0

Relevé du 23/11/2026 sur la 0.8.1 (`b97efa0`), demandé par l'utilisateur : parcours de Revel en utilisateur
venu de Framer (niveau débutant à intermédiaire), **en mode sans classes**, création d'un site d'essai,
comparaison avec Framer. Aucune modification de Revel n'a été faite pendant l'audit.

- Relevés bruts, au fil du parcours : [captures/audit-1.0/releves.md](captures/audit-1.0/releves.md).
- Captures : `docs/captures/audit-1.0/`.
- Gravité : **B** bloquant, **G** gênant, **M** mineur.

## 1. Ce qui a été parcouru

| Zone | Couvert |
| --- | --- |
| Accueil, Nouveau site, Importer (dialogue), Profil, Réglages de Revel (10 rubriques) | inventaire complet |
| Éditeur : barre d'outils, panneaux Pages, Calques, Ajouter, Composants, Médias, Styles, Interactions, Classes, Variables, CMS | inventaire complet |
| Volet Ajouter : 42 outils | chacun posé, réglages propres relevés |
| Vues CMS, Localisation, Plan du site, Paramètres (Général, Publication, Navigation, Sauvegarde), paramètres de page | inventaire complet |
| Site d'essai « Atelier Essai » | 4 pages, en-tête et pied communs, 9 blocs et outils sur l'accueil, 10 médias, 2 composants (propriétés, variante de survol), collection de 9 champs et 5 contenus, modèle des pages, liste de collection, filtres, formulaire, seconde langue, point de rupture, aperçu, génération (22 fichiers) |

Un second passage, le même jour, a couvert les zones laissées de côté (§ 11). Restent non essayés : le
dialogue d'import de l'interface et les imports par lien, Figma, WordPress et dépôt Git ; un assistant IA
réel branché sur le serveur ; la traduction automatique (environ 600 Mo à télécharger par langue) ; la galerie
de templates de l'interface ; le glisser d'un outil du volet Ajouter vers le canevas ; l'application de
bureau réelle (l'audit a tourné sur l'interface reliée au cœur de Revel par le pont d'essai, et sur la ligne
de commande). Le thème clair n'a été vérifié que sur l'éditeur.

## 2. Pannes

| # | Gr. | Constat | Où |
| --- | --- | --- | --- |
| P1 | **B** | Après un ajout, un changement ou un retrait d'animation, désélectionner dans les 350 ms fait planter toute l'interface (« reactive value … already disposed », puis plus aucun bouton ne répond). Cause : aperçu différé `set_timeout(preview, 350 ms)` qui lit un signal du panneau détruit (`interactions.rs` 1006, 1025, 967). | Interactions |
| P2 | **B** | Un composant créé à partir d'une carte de liste de collection garde ses lectures directes du contenu ; aucune propriété n'est créée ; l'utilisation n'a ni « Contenu » ni propriétés. Posée ailleurs, la carte n'offre ni collection ni contenu à choisir. La refonte D-191 ne couvre que les anciens composants à propriété « Contenu ». | Composants × CMS |
| P3 | G | Trois info-bulles montrent une clé de traduction : `anim-d-appear-unveil`, `anim-d-hover-follow`, `anim-d-scroll-drops`. | Interactions |
| P4 | G | La variante de survol d'un composant est écrite sur la classe de son élément (`.bloc-avantage:hover`) : les éléments voisins de même classe, hors composant, prennent aussi le survol (vérifié sur le site généré). | Variantes |
| P5 | G | Un champ « Choix » s'affiche sur le site par sa valeur enregistrée (« site-web », « identite ») et non par son libellé. | CMS |
| P6 | G | Le compteur du carrousel passe à « 1 / 0 » dans le canevas après le choix des images (correct sur le site généré). | Carrousel |
| P7 | M | « Révélation » recouvert par l'icône de liaison dans « Fonction : Image / Révélation ». | Image |
| P8 | M | Le panneau CMS de l'éditeur nomme la collection « projets_2 ». | Panneau CMS |
| P9 | M | Menu « Tri » d'une liste : « Date » deux fois. Menu « Champ » : deux entrées « Temps de lecture de « … » indiscernables. | Liste, Champ CMS |
| P10 | M | Clic droit sur un média : la fiche s'ouvre sous le menu ; Échap ne la ferme pas. | Médias |
| P11 | G | Poignée de largeur sur l'image du bloc « Texte et image » : la largeur est écrite, l'image ne change pas (la mise en page du bloc l'emporte). À confirmer à la main. | Canevas |
| P12 | M | Dialogue « Sauvegarder » : le champ Description n'a pas le curseur à l'ouverture. | Sauvegarde |
| P13 | M | Mode avec classes : après l'ajout d'une classe, la pastille combinée annonce « 0 élément ». | Styles |

## 3. Le mode sans classes n'est pas tenu jusqu'au bout

Le réglage masque le champ Classes, l'onglet Classes et le bouton d'état. Il reste :

| Gr. | Constat |
| --- | --- |
| **G** | Les styles restent partagés par classe : régler le sous-titre d'un bloc règle les quatre sous-titres de la page. Le panneau dit « Ordinateur · 4 éléments », sans dire lesquels ni proposer « seulement celui-ci ». |
| G | Noms de calques et étiquettes du canevas tirés des classes : « Bloc inner », « Bloc sous titre », « Champ libelle », « Formulaire etat », « Bloc hero ». |
| G | « Sommaire › Zone » liste des classes (`.intro`, `.page`) ; « Actions » d'Interactions vise des éléments par « Classe « bloc__inner » » et propose « Ajouter une classe » par défaut. |
| G | Paramètres de page : « Classe du corps de page ». Styles : « Code CSS (avancé) », « Classes selon le contenu », info-bulle « .titre — Ordinateur (base) ». |
| G | Aucun réglage d'aspect au survol sur un élément ordinaire (couleur d'un bouton) : seulement les 9 modèles d'Interactions › Survol, ou passer par un composant et sa variante de survol. Rien ne l'indique. |
| G | Les blocs fournis contiennent 10 réglages « écrits en code » que le panneau Styles ne règle pas (pied du panneau Classes). |

Voies possibles :
- **a.** Styles propres à chaque calque en mode sans classes (les classes des blocs deviennent des styles
  copiés à la pose). Contrepartie : plus de réglage commun aux sous-titres ; il faut un autre partage
  (styles de texte nommés, composants).
- **b.** Garder le partage, mais le montrer : « Style partagé par 4 éléments » avec leur liste et un bouton
  « Détacher ». Contrepartie : la notion de classe revient sous un autre nom.
- **c.** Retirer le mode sans classes et assumer un modèle par classes. Contrepartie : écart avec Framer,
  cible déclarée.

## 4. À modifier (logique d'un outil ou d'un écran)

| # | Gr. | Constat | Piste |
| --- | --- | --- | --- |
| M1 | G | L'insertion est un onglet secondaire (« Ajouter », derrière « Calques ») ; le « + » de la barre d'outils gère les points de rupture. | bouton d'insertion en tête de barre ; autre icône pour les points de rupture |
| M2 | G | Un bloc « section complète » posé pendant qu'une section est sélectionnée se place dans cette section. | le poser après la section |
| M3 | G | Outils posés hors bloc (liste, carrousel, défilement, onglets) : pleine largeur, sans marge ; seuls les blocs ont marges et largeur de contenu. | les poser dans une section à marges, ou donner les mêmes marges |
| M4 | G | « Champ CMS » posé hors liste ou modèle : paragraphe littéral « Champ CMS », sans message (il part sur le site et dans les textes à traduire). | désactiver hors contexte, ou demander la collection |
| M5 | G | Filtres de liste : boutons « Tout / Option 1 / Option 2 » et valeurs à taper ; rien ne les relie à un champ ni à la liste ; posés dans la liste, ils deviennent une case de la grille. | choisir la liste et le champ ; boutons créés d'après les valeurs |
| M6 | G | Barre de navigation : « Ajouter un lien » duplique le dernier lien ; pas de renommage dans la liste (Onglets, Accordéon et Filtres en ont un) ; liens d'origine tous vers « / ». | un seul modèle de liste pour tous les outils |
| M7 | G | Formulaire : aucun outil pour ajouter un champ (il faut en dupliquer un) ; pas de liste déroulante ; message d'envoi et destinataire communs à tout le site. | « + champ » sur le formulaire sélectionné |
| M8 | G | Relier à une propriété : rubrique « Relié aux propriétés » séparée du champ ; propriétés nommées « Texte », « Texte 2 » ; une icône ne se relie qu'à sa visibilité. | bouton de liaison à côté de chaque champ ; nom d'après le calque ; type Icône |
| M9 | G | Le H1 du modèle Vide est lié au titre du site : le modifier change le nom du site partout, sans avertissement. | titre écrit par défaut, ou signalement à la saisie |
| M10 | G | Fiche d'un contenu : le corps « Texte » (Markdown) n'est pas dans la liste des champs ; un champ Texte riche du même nom donne deux éditeurs « Texte ». | montrer le corps dans « Champs » |
| M11 | G | Première publication : hébergement SFTP ou GitHub à régler avant tout. | assistant de première publication (voir § 8) |
| M12 | M | Double-clic : pose dans « Ajouter », ouvre dans « Composants ». Double-clic à côté des lettres d'un texte : rien. Ctrl+B / Ctrl+I sans effet pendant la saisie. | à aligner |
| M13 | M | Zones En-tête et Pied créées avec un paragraphe d'attente qui part sur le site ; défilement : les images s'ajoutent après quatre éléments vides. | pas de contenu d'attente publié |

## 5. À ajuster (libellés, rangement, cohérence)

**Un même objet, plusieurs noms**

| Objet | Noms rencontrés |
| --- | --- |
| Boîte | outil « Conteneur », calque « Bloc », affichage « Bloc » |
| Pied | calque « Pied », panneau et fil d'Ariane « Pied de page » |
| Accordéon | outil « Accordéon », réglage « Volets », calques « Dépliant » |
| Interrupteur | panneau « Interrupteur », canevas « Bouton » |
| Accroche | bloc « En-tête de page », zone « En-tête », canevas « Bloc hero » |
| Champ CMS | rubriques « Champs CMS » et « Champ CMS » l'une sur l'autre ; calque « Paragraphe » jusqu'au rechargement |
| Liste | « Liste de collection » et « Liste · Projets », même panneau ; « Liste · Locales » |

**Un même réglage, plusieurs endroits**

| Réglage | Endroits |
| --- | --- |
| Nom du site | Général › Nom, Informations du site › Titre, Référencement › Nom affiché |
| Description, langue | Informations du site et Référencement ; Informations du site et Identité |
| Source d'une vidéo | « Vidéo (adresse) » et « Source » dans le même panneau |
| Média au survol | « Fonction : Survol » et « Change avec l'élément survolé » |
| Niveau de titre | « Niveau H1…H6 » et « Balise : Titre 1…6 » |
| Masquer | œil du calque et « Affichage : Masqué » |
| Effet au survol | Interactions › Survol, variante de survol, Effets › Transitions |
| Texte de l'interrupteur | « Texte quand allumé » et « Textes › État » |
| Profil | bulle Profil et Réglages › Profil et comptes |
| Filtre de liste | « Seulement si le champ » et « Filtre » |
| Liens du menu | liste « Liens » de la barre de navigation et Paramètres › Menus du site |
| Publier | barre du haut et carte de l'hébergement |

**Contrôles**
- Trois dessins de oui / non (case à cocher, interrupteur, ligne à case) dans les mêmes panneaux et dans
  Paramètres.
- Interrupteur dessiné comme un trait à butée : peu lisible (capture `tool-insert-video.png`).
- Proportions : deux listes différentes (Styles, Carrousel). « Menu replié » propose 479 px, absent des
  points de rupture du site.
- Rôles « Consentement aux cookies » et « Interrupteur » montrés sur tout bouton.

**Rangement**
- Ajouter : « En-tête », « Pied », « Sélecteur de langue » sous « CMS » ; « Lien », « Bouton » sous « Texte ».
- Réglages › Éditeur : « Classes », « Médias lourds », « Ouvrir en canevas libre » sous le titre
  « Recherche rapide ».
- Paramètres › Publication : « Template » et « Code personnalisé ».
- Styles › Effets : « Curseur », « Clics », « Empilement ». Styles › Texte : deux lignes « Coupure ».
  Styles › Bordure et Texte : deux « Contour ».

**Jargon visible** : « Flex », « Enfant flex », « Modifier le flex parent », « 3.5vw », « Image
(expression) », « Nom dans les fichiers », « .pages.yml », « content/projets », « Type au partage
(og:type) », « Propriétés du gabarit », « Paramètre d'adresse », « Dépôt », « Données ».

## 6. À remplacer

| Élément | Par |
| --- | --- |
| Deux modèles de liste d'éléments (navigation, carrousel, défilement / onglets, accordéon, filtres) | un seul, avec renommage, ordre, duplication, retrait |
| Champ texte « Image (adresse) » + icône | vignette cliquable (choisir, déposer, remplacer) |
| Panneau « Variables » (variables de données de sites importés) | le nom « Variables » pour les couleurs et tailles du site, ou retrait du panneau |
| Menu « Courbe » à 4 choix | à étudier : ressort et courbe libre |
| Tuile de template tirée au hasard dans « Nouveau site » | choix fixe |

## 7. À retirer ou à replier

| Élément | Raison |
| --- | --- |
| Outils « Grille » et « Colonnes » à côté de « Conteneur » | même boîte, trois affichages ; rendu identique à la pose |
| Outils « Horloge », « Toile de pixels » | niche, au rang de la navigation |
| Onglets : « Au survol », « Touches du clavier », « Son au changement » ; Interrupteur : « Musique », « Sons » | niche (menu de jeu) |
| Boucle › « Bandeau » | double l'outil « Défilement » |
| Image : « Révélation », « Média au survol » | niche en tête de panneau |
| Styles › Texte : retour à la ligne, coupures, mots longs, tronquer, lignes max, sens d'écriture, contour | six réglages de coupure au premier niveau |
| Styles › Bordure : quatre lignes de contour | niche |
| « Emplacement du contenu » sur chaque élément d'un composant | notion avancée montrée partout |
| Champs CMS « Code », « Groupe de champs », « Blocs » | avancés, au rang de Texte et Image |
| Réglages › Éditeur › forme des liens entre variantes | cosmétique, en tête de rubrique |
| Paramètres de page : classe du corps, type au partage, propriétés du gabarit | techniques |

« Replier » = garder derrière « Plus » ou un mode avancé ; « retirer » = décision au cas par cas.

## 8. Écarts avec Framer

| Sujet | Framer | Revel |
| --- | --- | --- |
| Survol et appui sur un calque | effets Hover et Press sur un calque seul, avec remplissage et ombre | 9 modèles ; aspect libre seulement par variante de composant |
| Défilement | Scroll Animation, Transform, Speed, Variant | 7 modèles, réglage « Intensité » seul |
| Glisser | contraintes par section | aucun réglage |
| Champs de formulaire | texte, case, radio, liste déroulante, ajoutés par « + » sur le formulaire | 11 types, pas de liste déroulante, pas d'ajout |
| Après l'envoi | redirection, overlay, état du bouton | message commun au site |
| Champs du CMS | booléen, couleur, lien, date, option, référence, multi-référence… | 13 types ; ni couleur, ni lien, ni galerie |
| Propriétés de composant | texte, lien, couleur, image, visibilité, icône ; choix de variante sur l'utilisation | texte, image, lien, couleur, visibilité, liste ; pas d'icône |
| Fenêtres par-dessus la page | overlays | absent des Actions |

Revel fait plus que Framer sur : localisation hors ligne sans surcoût, plan du site avec adresses sans page,
onglets, accordéon, sommaire, bandeau cookies, panier, modèle 3D, tailles fluides, export du site,
redirections à la volée, code du site en clair.

## 9. Ce qui tient

Création de page en ligne ; fenêtre de choix d'image ; nuancier et couleurs du site ; points de rupture ;
écran du composant, variantes et propriétés en un clic ; Champ CMS dans le modèle des pages ; tableau des
contenus ; Localisation ; aperçu ; avertissements de la génération ; recherche rapide.

## 10. Décisions à prendre

1. Mode sans classes : voie a, b ou c (§ 3). Elle commande les noms de calques, le survol, les Actions.
2. Composant et CMS (P2) : propriétés créées à la création du composant, ou seulement à la demande.
3. Volet Ajouter : quels outils restent au premier niveau (§ 7).
4. Première publication : assistant, hébergeur proposé, ou état actuel.
5. Vocabulaire : une liste de noms par objet (§ 5) à figer avant la 1.0.
6. Ordre : pannes B (P1, P2), puis § 3, puis § 4, puis § 5 à 7.

## 11. Second passage

| Zone | Essai | Résultat |
| --- | --- | --- |
| Import HTML | site de 3 pages importé par `revel import`, rouvert dans l'éditeur, régénéré et comparé à l'original | fidèle (hauteur, tailles, couleurs, grille) ; en-tête et pied reconnus, barre de navigation refaite, 3 cartes devenues un composant à 4 propriétés ; seul écart : l'espace entre les liens du menu. `import-check` annonce pourtant « 0/3 pages équivalentes » ; calques nommés d'après les classes d'origine (« A top », « Btn ») |
| Templates | 5 des 27 templates créés et générés | sans erreur ; niveau visuel au-dessus des blocs du volet Ajouter (capture `f06-tpl-shop.png`) |
| Lottie, modèle 3D | fichiers importés, choisis, générés | fonctionnent dans le canevas et sur le site (capture `site-boutique.png`) |
| Panier | ajout, quantités, retrait, total, rechargement | fonctionne ; pas d'outil « pastille de panier » pour l'en-tête (le template Boutique en a une) |
| Sauvegarde | version créée, comparée, restaurée | fonctionne ; le détail montre des fichiers techniques et le JSON du site ligne à ligne (illisible pour un débutant) |
| Publication SFTP | envoi vers un serveur d'essai local | aperçu, envoi de 102 fichiers, second envoi limité au fichier changé |
| Mettre en ligne | dialogue relevé | 6 hébergeurs, « Autre », message de l'hébergeur à coller : lisible (capture `f09-hebergement.png`) |
| Assistants IA | serveur lancé, actions listées | 31 actions, clé exigée, description OpenAPI |
| Glisser-déposer | déplacer, Alt+glisser | fonctionnent |
| Mode avec classes | états, classe ajoutée, panneau Classes | états riches (Survol, Appui, Focus clavier, Page actuelle, survol d'un parent) ; le mode sans classes n'a pas d'équivalent pour « Page actuelle », « Focus clavier » et le survol d'un parent |

Ajouts aux listes :
- § 3 (mode sans classes) : pas d'état « Page actuelle » pour le lien du menu de la page affichée.
- § 4 : pastille de panier à offrir comme outil ; détail d'une version à résumer en clair.
- § 5 : message de `import-check` à reformuler.

## 12. Historique git : clés et mots de passe

Recherche dans les lignes ajoutées par les 233 commits (toutes branches et étiquettes) : clés privées, jetons
(GitHub, AWS, OpenAI, Anthropic, Slack, Google, Figma), JWT, adresses avec identifiants, affectations de mot
de passe, fichiers `.env`, `.pem`, `.key`. **Aucun secret trouvé.** Seules valeurs relevées : jetons factices
du simulateur et des tests, noms de secrets cités sans valeur dans `release.yml`, identifiant public de
l'application OAuth. Auteurs des commits : deux adresses « noreply » de GitHub.

Hors secrets : `contact@alunites.com`, `site@alunites.com`, le domaine `alunites.com` et le numéro
« +33 4 72 27 03 74 » servent d'exemples dans des tests, le simulateur et une info de l'interface
(`inspector.rs:826`). À remplacer par des valeurs fictives s'il s'agit des vraies coordonnées.

Contrôle fait par expressions régulières ; gitleaks n'a pas pu être installé dans la session. Un passage de
gitleaks ou de l'analyse de secrets de GitHub confirmerait.
