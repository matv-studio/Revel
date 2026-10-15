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
  Élargie par D-31 (tout assistant, pas seulement Claude).

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

**D-22 — 15/10/2026 — prise (demande de l'utilisateur).** **Parcours de l'application et canevas comme Figma,
Framer et Webflow** :
- plus de panneau des sites ni de fil d'Ariane : barre supérieure à onglets de vue raccordés au plateau
  (*Éditeur*, *CMS*, *Localisation*, *Paramètres* pour un site ; *Sites*, *Réglages* à l'accueil) ; un clic sur
  un site ouvre l'éditeur ; vue d'ensemble, génération et publication réunies dans les paramètres du site
  (rubriques à la Webflow) ; vue CMS en tableau (lecture, édition à l'étape 3) ; localisation prévue à l'étape 3 ;
- canevas : la vue se déplace et se zoome sans remettre la page en page (espace + glisser, bouton du milieu,
  molette autour du cadre, zoom autour du pointeur, raccourcis de Figma) ; éléments déplacés à la souris (Alt :
  copie) et ajoutés par glisser-déposer depuis « Ajouter », emplacement montré selon les règles de Webflow ;
  parcours de la sélection au clavier ; copier/coller, envelopper/retirer l'enveloppe, menu contextuel ;
- nouveau site : dossier proposé dans `Documents/Revel` d'après le nom ; la fenêtre du système ne s'ouvre qu'à la
  demande. Pointeur invisible signalé dans cette fenêtre sous Windows : défaut connu de WebView2 152 avec le
  réglage « Masquer le pointeur pendant la saisie » quand la saisie précède l'ouverture d'une fenêtre
  (https://github.com/MicrosoftEdge/WebView2Feedback/issues/5687, corrigé dans WebView2 155) ; le nouveau parcours
  évite la saisie juste avant la fenêtre.

**D-23 — 15/10/2026 — prise (choix de l'utilisateur).** **Langues : un dossier par langue, comme Webflow
Localization** : contenus et données de chaque langue dans leur propre dossier (`content/en/…`, `data/en/…`),
adresses préfixées (`/en/…`, la langue principale sans préfixe), balises `hreflang` générées. Fichiers simples,
lisibles dans Pages CMS ; Revel garde les structures des langues alignées (étape 3.8).

**D-24 — 15/10/2026 — prise (choix de l'utilisateur).** **Polices copiées dans le site** : les fichiers de police
(importés ou téléchargés depuis Google Fonts par Revel) sont servis par le site lui-même ; aucune requête des
visiteurs vers Google (RGPD).

**D-25 — 15/10/2026 — prise (choix de l'utilisateur).** **Synchronisation GitHub à la publication** : Revel
enregistre (commit) et envoie les fichiers du site sur son dépôt quand on publie, ou à la demande ; il récupère
avant les enregistrements faits ailleurs (Pages CMS).

**D-26 — 15/10/2026 — prise (demande de l'utilisateur).** **Aperçu sans génération, « Générer » devient
« Exporter »** : l'aperçu ouvre le site tel qu'il sera publié, pages rendues à la demande depuis le document en
cours (protocole `revel-preview`, sans repères ni script de l'éditeur, scripts du site actifs ; identique à la
génération, vérifié par `canvas-check`), à la page affichée dans l'éditeur ; plus rien à générer avant. La
publication génère elle-même. L'ancienne rubrique « Génération » devient « Export » : fichiers du site copiés
dans un nouveau dossier choisi (héberger ailleurs, archiver), avec « Afficher le dossier ».

**D-27 — 15/10/2026 — prise.** **Schéma des collections : `.pages.yml` de Pages CMS, sans copie.** Revel lit les
champs, libellés, aides, valeurs par défaut, champs cachés, choix, références, objets et la présentation des
tableaux dans le fichier de configuration de Pages CMS (https://pagescms.org/docs/configuration/), et y écrira les
modifications de champs (3.3, texte modifié sur place, commentaires gardés). Une seule source : Revel et Pages CMS
décrivent toujours les mêmes contenus. Sans `.pages.yml`, les champs sont déduits des contenus ; le fichier est
créé à la première modification de champs. Dates écrites au format déclaré (`options.format`, jetons de date-fns
comme Pages CMS) ; références résolues par le nom de collection de `.pages.yml` et ses modèles (`{fields.slug}`) ;
nom d'un nouveau contenu tiré du modèle `filename`. Les collections d'Eleventy qui lisent un même dossier
(`offres`, `toutesOffres`) apparaissent une seule fois dans la vue CMS. Une collection décrite par Revel l'est
d'après ses contenus : types déduits des valeurs, format des dates existantes repris, variables d'Eleventy
(`layout`, `permalink`…) déclarées masquées, noms refusés par Pages CMS laissés hors du schéma ; un fichier créé
par Revel porte `settings.content.merge: true`, sans quoi Pages CMS retire d'un contenu qu'il enregistre les
valeurs absentes du schéma (vérifié dans son code, `app/api/.../files/[path]/route.ts`). Le nom d'un champ (clé
écrite dans les contenus) ne change pas après sa création, comme le nom d'API d'un champ Webflow ; retirer un
champ du schéma laisse ses valeurs dans les contenus.

**D-28 — 15/10/2026 — prise (demande de l'utilisateur).** **Panneau Variables.** Les affectations `{% set %}`
(variables de travail des gabarits, de la page et des composants : titre de la page pour le référencement, liste de
projets mis en avant…) quittent les calques, où elles n'ont pas de place visible, pour un panneau à part. La valeur
affichée est celle du rendu du canevas (relevée par le moteur, première exécution, signalée quand elle change d'un
passage de boucle à l'autre). L'expression se modifie sur place ; le cœur vérifie sa syntaxe avant de l'appliquer
(aussi pour les conditions et les boucles), pour que la page reste affichable ; les variables et filtres inconnus
restent acceptés (résolus au rendu).

**D-29 — 15/10/2026 — prise (demande de l'utilisateur).** **Dossiers des médias et des composants : rangement de
l'éditeur.** Comme les dossiers de médias et les groupes de composants de Webflow
(https://help.webflow.com/hc/en-us/articles/33961269934227, https://webflow.com/updates/component-groups), les
dossiers rangent sans déplacer : ils sont enregistrés dans `site.revel.json` (`library`), aucun fichier ne bouge et
aucune adresse ne change, donc aucun lien de page, de feuille de style ou de contenu du CMS ne casse. Les médias
listés sont les fichiers des dossiers copiés tels quels (hors code), filtrables par type (images, vidéos, sons,
documents, polices, animations) et triables. Ranger les fichiers sur le disque (et réécrire les liens) reste
possible plus tard avec l'import de médias (étape 3).

**D-30 — 15/10/2026 — prise (demande de l'utilisateur).** **Modèle de collection et design propre à un contenu.**
Les contenus d'une collection utilisent le gabarit de la collection (le modèle, comme le « Collection template » de
Webflow) : le modifier en affichant n'importe quel contenu change tous les contenus, ce que la barre d'outils
indique. Un contenu peut recevoir un design propre : copie du modèle (nouveaux identifiants, même gabarit parent,
donc l'en-tête et le pied du site restent partagés) enregistrée comme gabarit du site, et clé `layout` dans
l'en-tête du contenu (mécanisme d'Eleventy, déjà lu par Revel). Création et retour au modèle sont annulables en une
étape (suite de commandes appliquée par le cœur, fichiers compris, atomique). Problèmes traités : avec un schéma
Pages CMS qui ne garde pas les clés non déclarées, la clé `layout` y est déclarée en champ caché (Pages CMS garde
les valeurs des champs cachés : son formulaire part de tous les champs du schéma, `initializeState` dans
`lib/schema.ts`, et react-hook-form inclut ces valeurs dans les données enregistrées,
https://www.react-hook-form.com/api/useform ; un contenu sans design propre peut y recevoir `layout: ""`) ; une
clé vide reprend le modèle ; un gabarit introuvable reprend le modèle avec un avertissement à la génération ; un
design propre dont le contenu a disparu est signalé dans le panneau Pages et supprimable ; le gabarit n'est retiré
au retour au modèle que s'il ne sert à aucun autre contenu. Les modifications du modèle ne s'appliquent plus au
design propre (c'est son rôle) ; il n'y a pas de fusion automatique.

**D-31 — 15/10/2026 — prise (demande de l'utilisateur).** **Étape 4 : tout assistant IA, pas seulement Claude.**
Revel se pilote par n'importe quel modèle (Claude, GPT, modèle local) grâce au protocole MCP, devenu commun :
ChatGPT accepte des serveurs MCP en mode développeur (lecture et écriture, transports SSE et HTTP en flux,
https://www.infoq.com/news/2025/10/chat-gpt-mcp), l'API Responses et l'Agents SDK d'OpenAI aussi (stdio, HTTP),
Codex et l'application ChatGPT de bureau partagent leur configuration MCP (stdio et HTTP,
https://learn.chatgpt.com/docs/extend/mcp?surface=cli), LM Studio depuis 0.3.17 (serveurs locaux et distants,
`mcp.json`, https://lmstudio.ai/blog/mcp), Open WebUI depuis 0.6.31 (HTTP en flux seulement, utilisé avec Ollama,
https://docs.openwebui.com/features/mcp). D'où : `revel-mcp` en **stdio** et en **HTTP en flux sur la machine**
(127.0.0.1, jeton d'accès) ; accès à distance (connecteurs de ChatGPT et de claude.ai, qui n'atteignent pas la
machine directement) par tunnel avec authentification ; les mêmes commandes décrites en **OpenAPI** pour les outils
sans MCP. Les outils exposés sont les commandes de D-13 (annulables, visibles dans l'historique de Revel). Une
discussion intégrée à Revel, avec le fournisseur choisi (Anthropic, OpenAI ou un modèle local par une API
compatible OpenAI, comme celle d'Ollama, https://docs.ollama.com/api/openai-compatibility), reste à décider
(D-13 : pas de panneau de discussion pour l'instant).

**D-32 — 15/10/2026 — prise.** **Pages : conventions de référencement et adresses.** Une page porte un nom dans
Revel (`name`, sans effet sur le site), et son référencement dans `seo.title`, `seo.description`, `seo.image`
et `noindex` (conventions du site Alunites, reprises par le site de départ d'un nouveau site). Les paramètres de la
page écrivent ces valeurs là où la page les lit : variable `seo` de la page, ou fichier de données quand `seo` est
un chemin vers les données (`pages.agence.seo`, aussi modifiable dans Pages CMS ; clé ajoutée sur place,
commentaires gardés) ; une expression calculée n'est pas modifiable dans ces paramètres. `noindex` ajoute la balise
`robots` si le gabarit ne l'écrit pas et retire la page du plan du site, produit à la génération (avec
`robots.txt`) quand le site n'a pas ses propres pages à ces adresses. Une adresse se limite aux minuscules sans
accent, chiffres, `-`, `_`, `.` ; changer l'adresse d'une page publiée ajoute une redirection 301 (les
redirections qui menaient à l'ancienne adresse sont mises à jour). Les dossiers de pages sont des dossiers
d'adresse (`/recrutement/…`), gardés quand ils sont vides ; renommer un dossier change les adresses de ses pages
(avec redirections). Brouillon : page modifiable, non publiée. L'accueil (`/`) ne change pas d'adresse, ne se
supprime pas et reste publié.

**D-33 — 15/10/2026 — prise (demande de l'utilisateur).** **Étape 7 : compatibilité maximale avec Webflow,
Framer et Figma.** Les notions de Revel restent proches de celles de ces outils pour que l'import soit fidèle :
composants à propriétés par instance (Framer, Webflow), variables de design (couleurs, tailles, polices : Webflow,
Figma), points de rupture, collections du CMS, dossiers de pages. Sources d'import vérifiées : Webflow exporte le
code d'un site et son API de données lit collections, champs et contenus
(https://developers.webflow.com/data/reference/cms) ; Framer ne propose pas d'export HTML
(https://www.framer.com/help/articles/can-i-export-my-website-to-html-and-self-host-it/) : le site publié sera lu à
son adresse ; Figma donne le document en JSON et des rendus d'images par son API
(https://developers.figma.com/docs/rest-api/file-endpoints).

## Décisions ouvertes

**D-07 — à confirmer avant la première version publique.** Identifiant d'application `app.revel-studio.desktop`
(`tauri.conf.json`). Il détermine les dossiers de données et les mises à jour : le changer après diffusion casse la continuité.
Basé sur le domaine `revel-studio.app`, libre mais non enregistré.

**D-08 — ouverte.** Licence du code : propriétaire, MIT/Apache-2.0, GPL… Le dépôt est public depuis le 14/10/2026
(D-17) sans licence : tous droits réservés jusqu'à la décision. Une licence ouverte ne peut pas être retirée des
versions déjà publiées sous cette licence.

**D-09 — ouverte (étape 6).** Signature de code : certificat Windows (sinon avertissement SmartScreen), compte Apple Developer (sinon blocage Gatekeeper).

**D-34 — 15/10/2026 — prise.** **Composants à propriétés, modification isolée (comme Webflow).** Un composant
porte un nom affiché et des propriétés (`props` : nom d'expression, libellé, type texte, image, lien ou
visibilité, valeur par défaut) ; chaque utilisation (`Use`) donne ses valeurs en expressions (`args` : `"Titre"`,
`p.data.title`, `true`), évaluées là où le composant est utilisé, la valeur par défaut sinon ; dans le composant,
une propriété se lit comme une variable (les variables de l'endroit d'utilisation restent visibles, comme un
`include` de Nunjucks). Un clic dans une utilisation la sélectionne en entier (cadre vert) ; un double-clic ouvre
le composant (le reste de la page est estompé, « Terminer » le referme) ou modifie sur place le texte d'une de
ses propriétés pour cette utilisation. Dans le composant ouvert, un texte, une image, un lien ou la visibilité
d'un élément se relie à une propriété (nouvelle, valeur par défaut : la valeur écrite, ou existante) ; délier
réécrit la valeur par défaut. Détacher une utilisation la remplace par une copie de son contenu, valeurs écrites
dedans. Un composant ne peut pas être placé dans lui-même (refusé à l'édition, ignoré au rendu). Les propriétés
de Webflow (texte, texte riche, image, lien, vidéo, nombre, interrupteur, liste de collection) servent de
référence pour l'import de l'étape 7 ; la liaison d'une propriété à un champ de collection est faite (3.7),
les propriétés texte riche, vidéo et nombre viendront plus tard. Source : [Component properties — Webflow Help
Center](https://help.webflow.com/hc/en-us/articles/33961219350547).
