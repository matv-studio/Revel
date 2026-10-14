# Journal des décisions

Format : identifiant, date, statut (**prise**, **ouverte**, **à confirmer**), décision et raison.
Une décision prise n'est modifiée que par une nouvelle entrée qui la remplace.

## Décisions prises

**D-01 — 13/10/2026 — prise.** Nom « Revel ». Crate `revel` libre sur crates.io, paquet npm `revel` pris,
domaine `revel-studio.app` libre (vérifié par RDAP le 13/10/2026). Vérification INPI/EUIPO à faire avant diffusion publique.

**D-02 — 13/10/2026 — prise.** Coque Tauri 2, cœur en Rust. Moteur d'affichage du système
(WebView2, WKWebView, WebKitGTK 4.1).

**D-03 — 13/10/2026 — prise.** Espace de travail Cargo, une crate par module (voir ARCHITECTURE.md),
CI Windows/macOS/Linux. Points de retour : étiquette Git `step-N` à la fin de chaque étape
(convention choisie par l'utilisateur le 13/10/2026 ; remplace `etape-N`).

**D-04 — 13/10/2026 — prise.** Rust figé à 1.99.0 (`rust-toolchain.toml`), édition 2024.
Rust stable courant d'après `static.rust-lang.org/dist/channel-rust-stable.toml` (publié le 01/10/2026).

**D-05 — 13/10/2026 — prise.** Interface en **Leptos 0.8** (rendu côté client, WebAssembly) dans la vue web de Tauri,
construite avec **Trunk**. Critère de l'utilisateur : le plus complet, le plus de contrôle et de compatibilité.
- Le canevas d'édition doit afficher les sites exactement comme un navigateur : il faut un vrai moteur web.
  Le rendu natif de Dioxus (« Blitz ») est déclaré incomplet en CSS par Dioxus ; il ne convient pas au canevas.
- Leptos et Dioxus disposent tous deux d'un modèle officiel Tauri 2 (`create-tauri-app`, modèles `template-leptos`
  avec `leptos 0.8` et `template-dioxus` avec `dioxus 0.7`, vérifié le 13/10/2026). Garder Tauri conserve ses
  installateurs, son système de mise à jour et ses greffons ; Dioxus apporterait sa propre coque en doublon.
- Leptos : 5,2 millions de téléchargements (1,4 million récents) contre 3,1 millions (1,2 million) pour Dioxus (crates.io, 13/10/2026).
- Sources : https://github.com/tauri-apps/create-tauri-app (dossier `templates`), https://dioxuslabs.com/blog/release-070/, https://v2.tauri.app/start/frontend/leptos/

**D-06 — 13/10/2026 — remplacée par D-13.**

**D-10 — 13/10/2026 — remplacée par D-14 pour les pages fixes.** Import du cobaye en **mode mixte** :
- pages fixes (accueil, agence, contact, mentions…) importées depuis le HTML publié ;
- collections (projets, actualités, expertises, équipe, offres) importées depuis les fichiers Markdown/YAML
  et transformées en **collections CMS natives de Revel**, avec leurs gabarits recréés dans le modèle Revel.
- Critère de l'utilisateur : le moins de limites, le plus de contrôle. Ce mode n'impose aucun moteur Nunjucks
  et laisse tout éditable dans Revel.
- Généralisation : l'import HTML accepte n'importe quel dossier de site statique ou une adresse web ;
  l'import Markdown/YAML accepte tout dossier de contenus avec front matter (Eleventy, Hugo, Astro, Pages CMS).
- Le HTML de référence du cobaye est produit une fois par Eleventy (outil externe, uniquement pour l'import) ;
  Revel lui-même n'exécute jamais Node.

**D-14 — 13/10/2026 — prise (remplace D-10 pour les pages fixes).** Les pages fixes, gabarits et composants
du cobaye sont importés **depuis les gabarits Nunjucks** (convertis en arbre de nœuds Revel), et non depuis le HTML publié.
- Raison : importées depuis le HTML, les pages fixes figeraient leurs listes dynamiques (« dernières réalisations »,
  équipe, offres). Converties depuis les gabarits, elles restent liées aux collections. Le critère de l'utilisateur
  (le moins de limites) est mieux respecté.
- Moyen : `revel-markup` convertit HTML + balises Jinja/Nunjucks en nœuds (éléments, conditions, boucles,
  composants, images) ; les expressions sont évaluées par **minijinja 3.0.0** (moteur Jinja en Rust, par l'auteur de Jinja2).
- La logique JavaScript du site (filtres, collection « sitemap », environnements) est décrite dans
  `revel-import.json` (profil d'import), sous forme d'expressions : aucun code exécuté.
- Résultat mesuré le 13/10/2026 : 71 fichiers sur 71 (pages HTML, `sitemap.xml`, `feed.xml`, `robots.txt`, `.htaccess`)
  équivalents à la sortie Eleventy (outil `tools/compare-sites.py` ; seules les variantes d'images diffèrent,
  elles relèvent de l'étape 1.4).
- L'import depuis le HTML reste prévu comme importateur générique de sites statiques quelconques.

**D-11 — 13/10/2026 — prise.** Connexion GitHub depuis Revel par une **application OAuth GitHub avec autorisation
par code** (« device flow »), comme l'outil officiel `gh` ; jeton personnel accepté en solution de secours.
- Une application OAuth autorisée accède à tous les dépôts accessibles à l'utilisateur ; une GitHub App doit être
  installée dépôt par dépôt. Critère « le moins de limites » : application OAuth.
- GitHub recommande en général les GitHub Apps (permissions fines, jetons courts) : l'interface `revel-git`
  reste abstraite pour en ajouter une plus tard sans toucher au reste.
- Le device flow ne demande pas de secret client ; il doit être activé dans les réglages de l'application.
- Jetons conservés dans le trousseau du système (`keyring`), jamais dans les fichiers du site.
- Fait (13/10/2026) : application OAuth « Revel » sous matv-studio, device flow activé, jetons à expiration
  (8 h, renouvelés automatiquement avec le jeton de renouvellement, valable 6 mois sans usage).
  Client ID public intégré : `Ov23liuo9dnJsp95C633` (`revel-git::github::CLIENT_ID`).
- Droit demandé : `repo` (dépôts publics et privés, réglages GitHub Pages ; l'API Pages l'exige).
- Requêtes en JSON (acceptées par github.com) ; certificats racine du système (`rustls-platform-verifier`)
  pour fonctionner derrière un proxy d'entreprise ou un antivirus qui inspecte HTTPS.
- Sources : https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/differences-between-github-apps-and-oauth-apps,
  https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow

**D-12 — 13/10/2026 — prise.** Opérations Git avec **`git2` 0.21.0** (libgit2 compilée dans Revel, `vendored-libgit2`) :
clone, commit, push sans installation de Git. `gix` ne sait pas encore pousser (`crate-status.md` de gitoxide, 13/10/2026).
TLS : libgit2 utilise OpenSSL sous Linux, la pile du système sous Windows (WinHTTP) et macOS (Secure Transport).
Correction du 14/10/2026 : `libgit2-sys` dépend d'`openssl-sys` sur tous les Unix, macOS compris ; OpenSSL y est donc
compilé dans Revel (`vendored`, inutilisé par libgit2) pour que l'application ne dépende pas de Homebrew
(contrôle `otool -L` dans le workflow des installateurs).

**D-13 — 13/10/2026 — prise.** Pas de panneau de discussion Claude intégré pour l'instant.
La **liaison MCP est prévue dès maintenant** :
- toute modification du site passe par des **commandes** sérialisables (`revel-core`) ; l'interface, la ligne de commande
  et le serveur MCP appellent les mêmes commandes ; elles servent aussi à annuler/rétablir ;
- `revel-mcp` (étape 4) exposera ces commandes avec le SDK officiel `rmcp` (github.com/modelcontextprotocol/rust-sdk) :
  en local (stdio) pour Claude Desktop / Claude Code, et en HTTP pour un connecteur personnalisé claude.ai (avec tunnel).

**D-15 — 13/10/2026 — prise (demande de l'utilisateur).** Direction artistique propre à Revel, sans reprise
de celle d'Alunites : références d'usage Figma, Framer et Webflow ; interface en **gris neutres**, **un seul accent**
choisi dans *Réglages* (« Aucune » par défaut : action principale en contraste inversé) ; police **Geist** (OFL)
intégrée ; thèmes système, sombre, clair. L'interface est traitée comme le moteur : système de design dans
`docs/DESIGN.md`, captures de contrôle (`tools/ui-shots.mjs`) relues avant chaque livraison.

**D-16 — 13/10/2026 — prise.** Le cœur ne produit pas de phrases destinées à l'utilisateur : échecs et
avertissements sont des **clés de traduction avec paramètres** (`revel_protocol::Failure`, `HostWarning`), mises
en forme par `revel-i18n` dans la langue de l'interface (nombres, pluriels). Le futur serveur MCP renverra les
mêmes clés. Le message technique d'origine (système, serveur distant) reste joint en paramètre `detail`.

**D-17 — 14/10/2026 — prise (demande de l'utilisateur).** Dépôt `matv-studio/Revel` **public** : les minutes
d'Actions sont gratuites sur les machines standard de GitHub pour un dépôt public, alors qu'un dépôt privé dispose
de 2 000 minutes par mois (offre gratuite), macOS et Windows étant décomptés plus vite. Avant le passage en public,
l'historique a été réécrit (`git filter-repo`, seuls les commits concernés, signatures refaites) pour remplacer
l'adresse du serveur SFTP et l'identifiant OVH d'Alunites présents dans une donnée d'essai de l'interface ; les
étiquettes ont été reposées et les exécutions d'Actions qui renvoyaient aux anciens commits supprimées (workflow
*Maintenance*). Aucune licence pour l'instant : tous droits réservés (D-08 reste ouverte).
Sources : https://docs.github.com/en/billing/concepts/product-billing/github-actions,
https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository

**D-18 — 14/10/2026 — prise (choix de l'utilisateur).** **Styles détenus par Revel** (modèle Webflow) : la feuille
de style d'un site importé devient des styles du modèle (sélecteur, état, point de rupture, déclarations), dans
l'ordre d'origine ; Revel génère le CSS. Tout se modifie dans le panneau de styles ; le fichier d'origine n'est plus
écrit à la main.
- Options écartées : modifier directement le fichier CSS (comme les outils de développement d'un navigateur) ;
  garder le fichier tel quel et ajouter les styles de Revel par-dessus.
- Cascade des points de rupture comme Webflow : base sur ordinateur, modifications vers les largeurs plus étroites
  (https://help.webflow.com/hc/en-us/articles/33961300305811).
- Mise en œuvre (étape 2.1) : le CSS d'origine est lu sans réécrire les valeurs ; une règle ajoutée pour un point
  de rupture se place après les règles de base et des écrans plus larges (elle l'emporte sur elles, comme dans
  Webflow) et avant les règles du même sélecteur pour des écrans plus étroits.

**D-19 — 14/10/2026 — prise (choix de l'utilisateur).** **Textes liés modifiables sur la page et par le CMS** : un
texte issu d'un fichier de données ou d'un contenu (YAML, front matter) se modifie en cliquant dessus dans
l'éditeur ; Revel n'écrit que cette valeur dans le fichier, qui reste compatible avec Pages CMS. Les deux outils
éditent les mêmes fichiers ; Revel relit les fichiers modifiés hors de l'application. Synchronisation avec GitHub
(enregistrements de Pages CMS) : étape 3.

**D-20 — 14/10/2026 — prise (demande de l'utilisateur).** **Panneau de styles au niveau de Webflow, Framer et
Figma, pour un public non technique** : noms simples (le nom CSS reste en info-bulle), contrôles visuels plutôt que
saisie de valeurs (sélecteur de couleur à plan saturation-luminosité, curseurs de teinte et d'opacité, pipette ;
glisser sur un libellé pour changer un nombre ; boîte des marges ; boîte d'alignement 3 × 3 ; boutons à icônes),
CSS brut seulement dans une section « Code CSS » repliée. Pour chaque fonction, la version la plus claire des
trois outils (tableau et sources dans `docs/DESIGN.md`, section *Panneau de styles*). Les raccourcis CSS écrits
dans le site (`margin: 0 auto`, `border: 1px solid`, `background: #fff`) sont lus et modifiés sur place.

**D-21 — 14/10/2026 — prise (demande de l'utilisateur).** **Espace de travail modulable et code couleur de Webflow** :
- panneaux de l'éditeur (pages, composants, médias, calques, CMS, ajouter, styles, réglages, interactions) déplaçables
  par leur onglet comme dans After Effects : vers l'autre colonne, dans un autre groupe (en onglet, au-dessus ou
  au-dessous), en fenêtre flottante ; colonnes et groupes redimensionnables ; panneaux fermés et rouverts depuis le
  menu « Panneaux » ; Ctrl+\ masque tout (comme Figma) ; disposition par défaut rétablie en un clic. Le canevas
  reste au centre. Disposition gardée dans le stockage local de l'interface (préférence d'affichage, pas une donnée
  du site) ;
- onglets en texte seul (pas d'icônes), défilants quand ils ne tiennent pas, liste complète dans le menu du groupe ;
- fonctions prévues présentes dès maintenant, désactivées, l'étape en info-bulle (CMS, composants, médias : étape 3 ;
  interactions : étape 6) ;
- dans le panneau de styles, **bleu = valeur réglée ici, orange = valeur héritée**, comme Webflow
  (https://help.webflow.com/hc/en-us/articles/33961332425875) : couleurs fonctionnelles, en plus de l'accent unique
  de D-15 ; explications en info-bulles plutôt qu'en texte affiché.

## Décisions ouvertes

**D-07 — à confirmer avant la première version publique.** Identifiant d'application `app.revel-studio.desktop`
(`tauri.conf.json`). Il détermine les dossiers de données et les mises à jour : le changer après diffusion casse la continuité.
Basé sur le domaine `revel-studio.app`, libre mais non enregistré.

**D-08 — ouverte.** Licence du code : propriétaire, MIT/Apache-2.0, GPL… Le dépôt est public depuis le 14/10/2026
(D-17) sans licence : tous droits réservés jusqu'à la décision. Une licence ouverte ne peut pas être retirée des
versions déjà publiées sous cette licence.

**D-09 — ouverte (étape 6).** Signature de code : certificat Windows (sinon avertissement SmartScreen), compte Apple Developer (sinon blocage Gatekeeper).
