# Étape 1 — Ouvrir, construire et publier un site

Objectif : depuis Revel installé, ouvrir le site cobaye Alunites, le générer sans Node, le prévisualiser,
puis le publier par SFTP (OVH, recette) ou sur GitHub Pages, avec avertissements selon l'hébergeur.

**Critère d'acceptation principal :** pour les pages du cobaye, le site généré par Revel est équivalent à celui
d'Eleventy : mêmes URL, mêmes titres et descriptions SEO, mêmes redirections, mêmes médias. Un test automatique
compare les deux sorties (texte visible, balises `<title>`/`<meta>`, liste des fichiers, règles de redirection).

## Sous-étapes

Chaque sous-étape = commits + étiquette `etape-1.N`, pour revenir en arrière précisément.

| N | Contenu | Modules |
|---|---|---|
| 1.1 | **Format de projet sur disque** : dossier de site (`site.revel.json`, `media/`, `content/`, cache `.revel/` ignoré par Git), brouillons locaux, liste des sites récents | `revel-model`, nouveau `revel-project` |
| 1.2 | **Import du cobaye Alunites** (stratégie à choisir, voir D-10) : pages, SEO, redirections du `.htaccess`, médias, CSS/JS | `revel-import` |
| 1.3 | **Génération complète** : gabarits, CSS global, typographie française (espaces insécables, apostrophe), `sitemap.xml`, `robots.txt`, `feed.xml`, `.htaccess` généré depuis le modèle, mode recette (`noindex`, bandeau, pas d'Analytics) | `revel-render` |
| 1.4 | **Images** : AVIF/WebP/JPEG, largeurs 480–2400, cache par empreinte (les images inchangées ne sont pas recalculées) | `revel-assets` |
| 1.5 | **Aperçu local** : serveur HTTP intégré, aperçu dans une fenêtre de Revel, rechargement à chaque génération | `revel-app` |
| 1.6 | **Publication SFTP** : envoi des seuls fichiers modifiés (manifeste SHA-1, compatible avec celui de `deploy-sftp.sh` pour reprendre l'état actuel du serveur de recette), suppression des fichiers retirés, remplacement atomique, reprise après coupure ; mots de passe dans le trousseau du système | `revel-deploy` |
| 1.7 | **Publication GitHub Pages** : envoi du site généré sur une branche dédiée du dépôt ; avertissements (pas de 301 ni de PHP, dépôt public sur offre gratuite, 1 Go) ; fichier `CNAME` pour un domaine personnalisé | `revel-deploy`, `revel-git` |
| 1.8 | **Interface minimale** : ouvrir/importer, générer, aperçu, publier (choix de l'hébergeur, avertissements, progression), en anglais et en français | `revel-app`, `ui/` |
| 1.9 | **Test d'équivalence** Eleventy ↔ Revel dans la CI, installateurs de test (MSI, DMG, AppImage/deb), étiquette `etape-1` | CI |

## Choix à trancher

### D-10 — Stratégie d'import du site Alunites

| Option | Principe | Avantages | Inconvénients |
|---|---|---|---|
| **A. Depuis le HTML généré** | Eleventy construit le site une fois (hors Revel) ; Revel lit les pages HTML et les convertit en arbre d'éléments ; le non reconnu reste en « HTML brut » | Fidélité immédiate ; indépendant des gabarits Nunjucks ; réutilisable pour importer n'importe quel site statique | Les projets/articles deviennent des pages isolées : pas de collections CMS avant l'étape 3 ; structure répétée (en-tête, pied) à factoriser ensuite |
| **B. Depuis les sources Eleventy** | Revel lit Markdown + YAML + gabarits Nunjucks (moteur compatible Jinja en Rust, `minijinja` 3.0.0) | Garde les collections (40 projets, 23 actualités…) et le lien avec Pages CMS | Nunjucks et Jinja diffèrent ; filtres et shortcodes JavaScript du site (`image`, `frenchSpacing`…) à réécrire en Rust ; importateur propre à ce site |
| **C. Mixte** | Pages fixes (accueil, agence, contact…) depuis le HTML (A) ; collections depuis Markdown/YAML (B) avec gabarits recréés dans le modèle Revel | Collections éditables dès l'étape 1 ; pas de moteur Nunjucks | Le plus de travail ; deux chemins d'import à maintenir |

### D-11 — Connexion GitHub depuis Revel (publication GitHub Pages, contenus CMS)

| Option | Avantages | Inconvénients |
|---|---|---|
| **Autorisation par code (« device flow »)** : Revel affiche un code, l'utilisateur le valide sur github.com | Aucun jeton à copier ; pas de secret embarqué (non requis pour ce mode) | Nécessite d'enregistrer une application OAuth ou GitHub App au nom de Revel et d'y activer ce mode |
| **Jeton personnel** saisi par l'utilisateur | Rien à enregistrer côté Revel | Manipulation technique (création du jeton, droits, expiration) |

Source : https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow

### D-12 — Envoi Git (publication GitHub Pages)

Faits vérifiés : `gix` (gitoxide, 0.89.0) ne sait pas encore pousser (`crate-status.md` du projet, 13/10/2026).

| Option | Avantages | Inconvénients |
|---|---|---|
| `git2` 0.21.0 (libgit2) | Clone, commit, push ; aucune installation de Git requise | Bibliothèque C compilée avec Revel ; dépendances TLS selon le système |
| Git installé sur le poste | Comportement identique à Git | Contraire à « sans compétence technique » si Git est absent |
| API REST GitHub (création d'arbre et de commit) | Aucune dépendance Git | Une requête par fichier modifié ; lent pour les gros envois (4 537 fichiers au premier envoi) |

### Autres choix techniques (réversibles, module par module)

| Besoin | Options (versions crates.io du 13/10/2026) |
|---|---|
| SFTP | `russh` 0.64.1 + `russh-sftp` 3.0.1 (100 % Rust) · `ssh2` 0.9.6 (libssh2, C) |
| Images | `image` 0.25.10 + `ravif` 0.13.0 (100 % Rust, AVIF plus lent) · liaison à libvips (rapide, bibliothèque système à embarquer) |
| Lecture HTML (import) | `html5ever` 0.40.1 / `scraper` 0.27.0 (arbre complet) · `lol_html` 3.0.1 (réécriture en flux) |
| CSS | `lightningcss` 1.0.0-alpha.72 (pas de version stable publiée) |
| Mots de passe | `keyring` 4.2.0 (trousseau Windows, macOS, Linux) |
| Serveur d'aperçu | `axum` 0.8.9 + `tokio` 1.53.2 |

### D-05 — Cadre d'interface

Voir DECISIONS.md. L'interface de l'étape 1 est simple (listes, boutons, progression) : elle peut rester en HTML
sans cadre et être réécrite à l'étape 2, ou être faite directement dans le cadre choisi pour éviter une réécriture.

## Préalables côté utilisateur

- Accès GitHub de Claude aux dépôts `matv-studio/Revel` et `matv-studio/alunites-revel-lab`.
- Pour tester l'envoi SFTP réel : un dossier de recette distinct de `recette-v4k8/` (à créer chez OVH) et ses identifiants saisis par l'utilisateur dans Revel.
- Pour GitHub Pages : `alunites-revel-lab` public (c'est le cas), Pages activé par l'utilisateur dans *Settings > Pages*.
