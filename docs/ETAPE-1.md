# Étape 1 — Ouvrir, construire et publier un site

Objectif : depuis Revel installé, ouvrir le site cobaye Alunites, le générer sans Node, le prévisualiser,
puis le publier par SFTP (OVH, recette) ou sur GitHub Pages, avec avertissements selon l'hébergeur.

**Critère d'acceptation principal :** pour les pages du cobaye, le site généré par Revel est équivalent à celui
d'Eleventy : mêmes URL, mêmes titres et descriptions SEO, mêmes redirections, mêmes médias. Un test automatique
compare les deux sorties (texte visible, balises `<title>`/`<meta>`, liste des fichiers, règles de redirection).

## Avancement

| N | État au 13/10/2026 |
|---|---|
| 1.1 | fait : `revel-project` (ouverture, enregistrement, contenus, données, génération) |
| 1.2 | fait : `revel-import` (Eleventy) + `revel-markup` ; profil `revel-import.json` du cobaye |
| 1.3 | fait : `revel-render` ; 71/71 fichiers équivalents à Eleventy |
| 1.4 | fait : `revel-assets` (AVIF/WebP/JPEG/PNG, pas d'agrandissement, cache `.revel/img`) ; 3 843 variantes ; 1re génération 22 min sur 2 cœurs (Eleventy : 19 min), suivantes 2,5 s ; rendu visuel identique (captures comparées) |
| 1.6 | fait côté moteur : `revel-deploy::sftp` (manifeste compatible `deploy-sftp.sh`, renommage atomique, reprise, clé du serveur) ; essai réel contre OpenSSH en CI ; commande `revel deploy-sftp` |
| 1.0, 1.5, 1.7 à 1.9 | à faire |

## Sous-étapes

Chaque sous-étape = commits + étiquette `step-1.N`, pour revenir en arrière précisément.

| N | Contenu | Modules |
|---|---|---|
| 1.0 | **Commandes** (`revel-core`) : toute modification du modèle est une commande sérialisable, réversible ; base de l'annuler/rétablir et du futur serveur MCP | nouveau `revel-core` |
| 1.1 | **Format de projet sur disque** : dossier de site (`site.revel.json`, `media/`, `content/`, cache `.revel/` ignoré par Git), brouillons locaux, liste des sites récents | `revel-model`, nouveau `revel-project` |
| 1.2 | **Import du cobaye Alunites** (mode mixte, D-10) : pages, SEO, redirections du `.htaccess`, médias, CSS/JS | `revel-import` |
| 1.3 | **Génération complète** : gabarits, CSS global, typographie française (espaces insécables, apostrophe), `sitemap.xml`, `robots.txt`, `feed.xml`, `.htaccess` généré depuis le modèle, mode recette (`noindex`, bandeau, pas d'Analytics) | `revel-render` |
| 1.4 | **Images** : AVIF/WebP/JPEG, largeurs 480–2400, cache par empreinte (les images inchangées ne sont pas recalculées) | `revel-assets` |
| 1.5 | **Aperçu local** : serveur HTTP intégré, aperçu dans une fenêtre de Revel, rechargement à chaque génération | `revel-app` |
| 1.6 | **Publication SFTP** : envoi des seuls fichiers modifiés (manifeste SHA-1, compatible avec celui de `deploy-sftp.sh` pour reprendre l'état actuel du serveur de recette), suppression des fichiers retirés, remplacement atomique, reprise après coupure ; mots de passe dans le trousseau du système | `revel-deploy` |
| 1.7 | **Publication GitHub Pages** : envoi du site généré sur une branche dédiée du dépôt ; avertissements (pas de 301 ni de PHP, dépôt public sur offre gratuite, 1 Go) ; fichier `CNAME` pour un domaine personnalisé | `revel-deploy`, `revel-git` |
| 1.8 | **Interface minimale en Leptos** : ouvrir/importer, générer, aperçu, publier (choix de l'hébergeur, avertissements, progression), en anglais et en français | `revel-app`, `ui/` |
| 1.9 | **Test d'équivalence** Eleventy ↔ Revel dans la CI, installateurs de test (MSI, DMG, AppImage/deb), étiquette `step-1` | CI |

## Choix retenus (13/10/2026, voir DECISIONS.md)

- **D-10 import mixte** : pages fixes depuis le HTML publié, collections depuis Markdown/YAML vers des collections CMS natives.
- **D-11 GitHub** : application OAuth + autorisation par code (device flow), jeton personnel en secours, trousseau du système.
- **D-12 Git** : `git2` 0.21.0 avec libgit2 intégrée.
- **D-05 interface** : Leptos 0.8 (WebAssembly) dans Tauri 2, construit avec Trunk.
- **D-13** : pas de discussion Claude intégrée ; toutes les modifications passent par des commandes réutilisables par le futur serveur MCP.

## Bibliothèques retenues pour l'étape 1 (versions crates.io du 13/10/2026)

| Besoin | Choix | Raison |
|---|---|---|
| SFTP | `russh` 0.64.1 + `russh-sftp` 3.0.1 | 100 % Rust : aucune bibliothèque C à embarquer sur les trois systèmes |
| Images | `image` 0.25.10 + `ravif` 0.13.0 | 100 % Rust ; AVIF plus lent compensé par le cache par empreinte |
| Lecture HTML | `scraper` 0.27.0 (html5ever) | Arbre complet nécessaire pour reconstruire les éléments |
| Front matter / YAML | `serde-saphyr` 1.3.0 | `serde_yaml` n'est plus maintenu ; serde-saphyr est en Rust pur et maintenu |
| Expressions | `minijinja` 3.0.0 | Syntaxe Jinja, proche de Nunjucks ; moteur maintenu par l'auteur de Jinja2 |
| Mots de passe | `keyring` 4.2.0 | Trousseau Windows, macOS, Linux |
| Serveur d'aperçu | `axum` 0.8.9 + `tokio` 1.53.2 | Standard de l'écosystème |
| CSS | `lightningcss` 1.0.0-alpha.72 | Seule option complète ; version alpha à surveiller |

## Préalables côté utilisateur

- Étape 1.7 : enregistrer l'application OAuth « Revel » sur matv-studio (guide fourni à ce moment-là).
- Pour tester l'envoi SFTP réel : un dossier de recette distinct de `recette-v4k8/` (à créer chez OVH) et ses identifiants saisis par l'utilisateur dans Revel.
- Pour GitHub Pages : `alunites-revel-lab` public (c'est le cas), Pages activé par l'utilisateur dans *Settings > Pages*.
