# Étape 5 — Mise en ligne directe, noms de domaine, formulaires

Objectif : qu'une personne non technique mette son site en ligne depuis Revel aussi simplement que dans Webflow
ou Framer — un bouton « Publier », une première mise en ligne guidée chez son hébergeur, puis son nom de domaine
et les formulaires du site — sans service extérieur ajouté par Revel (D-36, D-44) : seul l'hébergeur choisi par
l'utilisateur intervient.

**Critères d'acceptation**
1. Le bouton « Publier » de la barre supérieure publie sans quitter la vue : panneau avec l'adresse du site, la
   date de la dernière publication, les pages modifiées depuis, une destination ou plusieurs (cases à cocher),
   l'avancement, puis « En ligne » et « Voir le site ». Un point signale des modifications non publiées.
2. Première mise en ligne : choix de l'hébergeur dans une liste (OVHcloud, o2switch, Infomaniak, Hostinger,
   IONOS, GitHub Pages) ou « Autre » ; l'endroit où trouver les accès est indiqué ; trois champs au plus
   (adresse du serveur, identifiant, mot de passe), remplis en collant le message de l'hébergeur.
3. Connexion testée aussitôt ; dossier du site proposé d'après le serveur ; clé du serveur enregistrée sans
   rien afficher la première fois, alerte seulement si elle change ; vocabulaire simple (site public, version de
   test), réglages techniques repliés.
4. Avant de publier : liens cassés, images trop lourdes, textes alternatifs manquants, pages sans titre,
   signalés en peu de lignes, avec « Publier quand même ».
5. Revenir à la publication précédente en un clic.
6. GitHub Pages en un clic : dépôt créé au nom du site, adresse affichée.
7. Nom de domaine : enregistrements DNS prêts à copier, vérification par Revel (domaine vers le site, HTTPS).
8. Formulaires du site reçus sans service extérieur (hébergements avec PHP) ; signalés indisponibles sinon.

## Sous-étapes

| N | Contenu | Modules |
|---|---|---|
| 5.1 | **Publier en un clic** : panneau du bouton « Publier » (adresse, dernière publication, destinations cochables, avancement, « Voir le site »), point des modifications non publiées, pages modifiées depuis la dernière publication | `revel-core`, `revel-ui`, `revel-app` |
| 5.2 | **Première mise en ligne guidée** : liste des hébergeurs et « Autre », où trouver les accès (chemin de l'espace client, lien), collage du message de l'hébergeur, trois champs | `revel-deploy`, `revel-ui` |
| 5.3 | **Vérifications automatiques** : test de connexion, choix du dossier du site par exploration du serveur, clé du serveur enregistrée sans affichage, vocabulaire simple | `revel-deploy`, `revel-core`, `revel-ui` |
| 5.4 | **Contrôles avant publication** : liens cassés, images trop lourdes, textes alternatifs, pages sans titre | `revel-core`, `revel-ui` |
| 5.5 | **Revenir à la publication précédente** | `revel-core`, `revel-deploy`, `revel-ui` |
| 5.6 | **GitHub Pages en un clic** : dépôt créé au nom du site | `revel-git`, `revel-core`, `revel-ui` |
| 5.7 | **Nom de domaine** : enregistrements DNS à copier selon l'hébergeur, vérification du domaine et du HTTPS | `revel-deploy`, `revel-ui` |
| 5.8 | **Formulaires** : réception des messages des formulaires du site par l'hébergement (PHP), sans service extérieur | `revel-render`, `revel-project`, `revel-ui` |

## Avancement

| N | État |
|---|---|
| 5.1 | Terminée : bouton « Publier » de la barre supérieure (panneau ancré : adresse, « Site public » / « Version de test », dernière publication, pages modifiées dépliables avec accès à la page dans l'éditeur, cases à cocher si plusieurs hébergements, avancement, « En ligne », « Voir le site », accès à compléter), point orange des modifications non publiées, publication de plusieurs hébergements à la suite (arrêt au premier échec), état partagé avec Paramètres › Publication (D-47) |
| 5.2 | Terminée : fenêtre « Mettre en ligne » (bouton du panneau vide, « Ajouter un hébergement ») : liste déroulante des hébergeurs (OVHcloud, o2switch, Infomaniak, Hostinger, IONOS, GitHub Pages) et bouton « Autre » à côté ; étapes pour obtenir les accès et bouton « Espace client » ; message de l'hébergeur collé (hébergeur reconnu d'après le serveur, champs remplis, message effacé car il contient le mot de passe) ; serveur, identifiant, mot de passe ; port, dossier et version (site public, version de test) repliés (D-48, `docs/HEBERGEURS.md`) |
| 5.3 | Terminée : « Se connecter » teste la connexion (délai de 20 s, message propre à o2switch), enregistre la clé du serveur sans l'afficher, liste les dossiers du serveur et propose celui du site (domaine du site, dossier habituel de l'hébergeur) ; « Publier » enregistre et publie ; clé affichée seulement si elle change (D-49) |
| 5.4 | Terminée : contrôles du site généré avant de publier (liens internes cassés, images de plus de 1 Mo, images sans texte alternatif, pages sans titre), regroupés et dépliables dans le panneau, page ouverte dans l'éditeur d'un clic, « Publier quand même » ; « Vérifier » les signale sans s'arrêter (D-50) |
| 5.5 | Terminée : bouton « Revenir » par hébergement (date de la publication précédente, confirmation) ; publications gardées dans le dossier du site, fichiers rangés par empreinte, deux versions par hébergement ; un second retour annule le premier (D-51) |
| 5.6 | Terminée : GitHub Pages choisi dans la liste : compte GitHub, dépôt proposé au nom du site (`compte/nom-du-site`), créé public à la première publication s'il n'existe pas (D-52) |
| 5.7 | Terminée : fenêtre « Nom de domaine » (Paramètres › Publication, icône globe) : enregistrements A, CNAME et AAAA de GitHub Pages prêts à copier, chemins de l'espace client pour les autres hébergeurs ; « Vérifier » : adresses du domaine, HTTPS, page d'accueil identique à la publiée ; domaine enregistré comme adresse du site (et domaine personnalisé de GitHub Pages) (D-53) |
| 5.8 | Terminée : bloc « Formulaire » (panneau Ajouter › Formulaires) ; Paramètres › Formulaires : adresse de réception, objet, message après l'envoi, hébergements qui reçoivent les formulaires ou non ; `revel-form.php` produit à la génération (champ piège, limite par adresse IP, envoi sans recharger la page, page de confirmation sans JavaScript) ; GitHub Pages signalé sans PHP (D-54) |

## Passe d'ergonomie après la 0.4 (demande du 16/10/2026)

Retours après installation de la 0.4 : rendre Revel évident pour une personne qui ne fait pas de sites
(comparaison avec Webflow, Framer et Figma), sans étape intermédiaire évitable.

| Sujet | Fait |
|---|---|
| Fonctions « prévues à l'étape 3 » encore affichées | Cause : bas du panneau CMS resté sur ses boutons prévus, panneaux CMS et Variables restés ouverts dans les dispositions enregistrées avant l'espace CMS. Bouton du panneau qui ouvre la vue CMS ; disposition mise à jour une fois (panneaux CMS et Variables fermés, rouvrables) |
| Nouveau site | Page d'accueil créée, feuille de style créée au premier style sans message, lien ajouté à la génération, gabarit de base pour une page d'un site sans gabarit ; fond sombre (par défaut) ou clair (D-56) |
| Calques | Ni `html`, `head`, `meta` ni scripts ; `body` à la racine ; texte sur la ligne de son élément ; noms simples (classe lisible ou nature de l'élément), balise en info-bulle ; mêmes noms dans le fil d'Ariane et le canevas ; ajout jamais dans `head` (D-55) |
| Canevas | Double-clic sur un texte de composant ; poignées de largeur du cadre ; marges et espacements en hachures, réglables en glissant ; Ctrl+Espace (D-58) |
| Interface | Icône des réglages en engrenage ; « Importer » au lieu de « Importer un projet Eleventy » ; listes déroulantes à la direction artistique de Revel (`appearance: base-select` là où le moteur la connaît, liste du système sinon) ; Réglages de l'application en rubriques (Général, Apparence, Traduction automatique, Assistants IA, Comptes) |
| Dossier des projets | Installation Windows dans `C:\Program Files\Revel`, sites dans `Projects` à côté de l'application, dossier modifiable dans Réglages › Général (D-57) |
