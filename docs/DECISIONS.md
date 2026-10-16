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
(D-13 : pas de panneau de discussion pour l'instant). *Précisée le 16/10/2026 : ni accès à distance (D-44),
ni discussion intégrée (D-45).*

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

**D-55 — 16/10/2026 — prise.** **Calques et noms pour les non-techniciens.** Les calques montrent ce que l'on voit
sur la page, comme Webflow (Navigator) et Framer : ni `html`, `head`, `meta`, `link`, `script`, `style`, `title`,
`br` ni doctype ; `body` à la racine (« Corps de page ») ; contenu de la page directement sous `body` quand le
gabarit n'ajoute rien autour ; descendants d'un SVG masqués. Le texte d'un élément est montré sur sa ligne, plus
discret, au lieu d'une ligne « texte » à part (un texte seul dans un bloc se déplace avec son bloc). Nom d'un
élément : sa classe principale rendue lisible (la dernière qui prolonge la première : `bloc bloc-hero` →
« Bloc hero », `hero__media` → « Hero media »), sinon sa nature dans la langue de l'interface (« Titre 1 »,
« Paragraphe », « Bloc »…) ; balise et classes en info-bulle ; icône selon la nature ; composants en vert. Mêmes
noms dans le fil d'Ariane et les étiquettes du canevas. Ajouter un élément quand `html`, `head` ou `body` est
sélectionné le place dans la page (au début ou à la fin selon la position de l'emplacement du contenu), jamais
dans l'en-tête du document ni dans le gabarit partagé.

**D-56 — 16/10/2026 — prise.** **Pas d'étape « feuille de style ».** Une feuille de style (`/assets/site.css`)
est créée au premier style posé, sans message ni bouton ; la génération ajoute son lien dans `<head>` s'il manque.
Une page créée dans un site sans gabarit reçoit un gabarit de base (doctype, `html` dans la langue du site,
`meta charset` et `viewport`, titre de la page, lien de la feuille de style, `body` avec l'emplacement du contenu).
Nouveau site : fond sombre (par défaut) ou clair, réglé par des variables CSS (`--fond`, `--texte`,
`--texte-doux`) modifiables ensuite.

**D-57 — 16/10/2026 — prise.** **Dossier des projets et installation dans « Programmes ».** L'installateur
Windows (NSIS) installe Revel pour tous les utilisateurs dans `C:\Program Files\Revel` (`installMode:
perMachine`, droits d'administrateur demandés une fois) et crée `Projects` à côté de l'application, ouvert en
modification au groupe « Utilisateurs » (`icacls … /grant *S-1-5-32-545:(OI)(CI)M`) ; la désinstallation ne
retire ce dossier que s'il est vide. Nouveaux sites créés par défaut dans `<dossier de Revel>\Projects\<nom>` quand
ce dossier est modifiable, sinon dans `Documents/Revel` (installation MSI, paquets Linux dans `/usr`) ; jamais dans
l'application sur macOS (paquet signé) ; AppImage : à côté du fichier. Dossier modifiable dans Réglages › Général.
Sources : [installateur Windows de Tauri](https://v2.tauri.app/distribute/windows-installer/),
[icacls](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/icacls).

**D-58 — 16/10/2026 — prise.** **Canevas : réglages directs.** Marges et espacements intérieurs survolés en
hachures dans la couleur de sélection de Revel ; sur l'élément sélectionné, un côté se règle en le glissant
(valeur affichée, Maj : les quatre côtés, Alt : le côté opposé, double-clic : valeur retirée), comme Framer.
Largeur du cadre réglable par des poignées sur ses bords (responsive horizontal, point de rupture suivi,
double-clic : largeur du point de rupture). Ctrl+Espace ouvre la palette de commandes comme Ctrl+K (sur macOS, Contrôle-Espace
sélectionne la source de saisie précédente quand plusieurs sont installées et n'atteint alors pas Revel :
[Apple](https://support.apple.com/guide/mac-help/mchlp1406) ; Ctrl+K reste). Un
texte lié à une donnée dans un composant (titre du site, lien du menu) se modifie au double-clic ; un texte fixe
d'un composant ouvre le composant et passe en modification du texte au même double-clic.

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

**D-35 — 15/10/2026 — prise.** **Langues (3.8), comme la localisation de Webflow.** Langue principale sans
préfixe, autres langues sous `/<code>/` (`Site::locales` : code BCP 47, nom affiché, publiée ou en préparation).
Les textes écrits dans les pages, gabarits et composants (textes, `alt`, `title`, `placeholder`, `aria-label`,
propriétés texte des composants, variables texte des pages) se traduisent par leur texte d'origine
(`Locale::texts`, espaces regroupés) : un même texte n'est traduit qu'une fois et la traduction ne dépend pas de
la place de l'élément ; un texte non traduit reste dans la langue principale. Données et contenus : dossier de la
langue (`_data/en/…`, `content/en/projets`), fusionnés sur ceux de la langue principale (clé par clé ; un
contenu non traduit reprend celui de la langue principale) ; le fichier de la langue est une copie créée à la
première traduction (annulable), et `.pages.yml` reçoit alors l'entrée de la langue (« Projets (EN) ») pour que
Pages CMS la modifie. L'adresse d'un contenu est calculée avec l'en-tête de la langue principale (même adresse
dans chaque langue, préfixe mis à part). Pages des autres langues : pages HTML seulement (plan du site, flux,
`robots.txt`, 404 : langue principale), `<html lang>` de la langue, liens internes `/…` préfixés (sauf fichiers
copiés et liens portant `hreflang`), liens `hreflang` vers chaque version (elle-même comprise) et `x-default`,
en adresses complètes (adresse du site requise), plan du site de Revel avec les pages de chaque langue publiée.
Typographie française seulement pour le français. Structure et styles sont communs à toutes les langues.
Sources : [Localization overview — Webflow Help Center](https://help.webflow.com/hc/en-us/articles/33961235675155),
[Localized versions of your pages — Google Search Central](https://developers.google.com/search/docs/specialty/international/localized-versions).

**D-36 — 15/10/2026 — prise.** **Indépendance : traduction automatique hors ligne.** Principe (demande du
15/10/2026) : Revel doit être l'outil le plus complet possible sans dépendre d'un prestataire extérieur ni d'une
connexion avant la mise en ligne du site ; une fonction ne doit pas devenir inutilisable parce qu'un service
change ses conditions (DeepL, Google). La traduction automatique utilise donc les modèles Marian OPUS-MT de
l'université d'Helsinki (licence Apache-2.0), exécutés sur le processeur par `candle` (`revel-translate` :
découpage SentencePiece et lecture des poids Flax écrits en Rust, sans Python). Français ↔ anglais livrés dans
l'installateur (ressources Tauri) ; 20 autres langues (vers et depuis l'anglais : allemand, espagnol, italien,
néerlandais, suédois, danois, finnois, tchèque, slovaque, hongrois, bulgare, russe, ukrainien, estonien,
catalan, chinois, arabe, hindi, indonésien, vietnamien) téléchargées une fois depuis les réglages, depuis Hugging
Face, aux révisions figées et empreintes SHA-256 vérifiées (`catalog.rs`, produit par
`tools/models-catalog.py`) ; ensuite tout fonctionne sans connexion. Entre deux langues sans modèle direct, la
traduction passe par l'anglais (allemand → français : allemand → anglais → français). Toute langue peut être
retirée : fichiers téléchargés supprimés ; langue livrée mise hors service (ses fichiers font partie de
l'installation, en lecture seule) et remise sans téléchargement. On traduit toujours depuis la langue
principale du site, seulement ce qui n'est pas encore traduit ; HTML, Markdown, expressions, adresses et mots
gardés (nom du site, liste du site) sont préservés ; chaque traduction est marquée « à relire »
(`Locale::machine`) jusqu'à ce qu'elle soit validée ou modifiée ; toute l'opération forme une seule étape
d'annulation. Qualité : bonne sur les phrases, faible sur les fragments isolés (« Mettre la vidéo en pause » →
« Breaking the video ») : d'où la relecture. Sources : [OPUS-MT — Helsinki-NLP](https://github.com/Helsinki-NLP/Opus-MT),
[modèles Helsinki-NLP — Hugging Face](https://huggingface.co/Helsinki-NLP), [Embedding additional files —
Tauri](https://v2.tauri.app/develop/resources/).

**D-37 — 15/10/2026 — prise.** **Réglages du site (3.9) : identité, code personnalisé, polices.** Nom,
adresse publique (`https://…`, sans `/` final, vérifiée) et langue principale (code valide, absent des autres
langues) se modifient comme toute commande (`SetSiteInfo`, annulable), avec la typographie française. Code
personnalisé (comme Webflow) : celui du site, puis celui de la page, ajoutés avant `</head>` et avant `</body>`
de chaque page HTML, après la typographie et les liens de langue (gardé tel qu'écrit) ; présent dans l'aperçu et
le site publié, jamais dans le canevas (scripts arrêtés). Polices (D-24) : une police du site est une famille
déclarée par des règles `@font-face` de la feuille de style ; Revel les liste (graisses, styles, plages des
polices variables, taille des fichiers, utilisation par `font-family`, `font` ou une variable CSS). Ajout depuis
Google Fonts : catalogue lu sur `fonts.google.com/metadata/fonts` (gardé un mois dans les réglages pour le
travail sans connexion), feuille de l'API CSS2, fichiers WOFF2 des graisses choisies et des seuls jeux de
caractères utiles aux langues du site (latin et latin étendu, plus cyrillique, grec ou vietnamien selon les
langues), copiés dans `<médias>/fonts/<famille>/` ; import de fichiers WOFF2, WOFF, TTF, OTF (contenu vérifié ;
graisse et style lus dans le nom du fichier, modifiables). Règles insérées après les `@import` (sinon ignorés).
Ajout et retrait sont une seule étape d'annulation chacun : les fichiers sont déplacés (`MoveFiles`) entre leur
place et `.revel/`, dossiers vidés retirés. Sources : [Custom code in head and body tags — Webflow Help
Center](https://help.webflow.com/site-settings/custom-code), [CSS API update — Google
Fonts](https://developers.google.com/fonts/docs/css2).

**D-38 — 15/10/2026 — prise.** **Versions du site et synchronisation GitHub (3.10, D-25).** Le dossier du
site est un dépôt Git ordinaire (`git2`, sans Git installé) ; « Activer les versions » le crée s'il n'existe pas,
ajoute `.revel/` (caches, corbeille) au `.gitignore`, garde les fins de ligne telles quelles dans ce dépôt
(`core.autocrlf` désactivé : sous Windows, Git les convertirait à chaque restauration ou récupération) et
enregistre une première version. Une version enregistre
tous les fichiers du site (ajoutés, modifiés, supprimés), avec un message (proposé : date et heure) et l'auteur
réglé pour Git (dépôt ou ordinateur), sinon le compte GitHub connecté, sinon « Revel ». Paramètres › Versions : changements en cours, puis versions
de la plus récente à la plus ancienne (fusions marquées) ; pour chacune, fichiers changés et comparaison ligne
par ligne (3 lignes de contexte, nombres de lignes ajoutées et retirées). Restaurer une version ne réécrit pas
l'historique : l'état actuel est d'abord enregistré (« Avant restauration »), puis les fichiers de la version
remis en place et enregistrés comme nouvelle version, comme les sauvegardes de Webflow ; le document ouvert est
relu (historique d'annulation vidé). Synchronisation avec `origin` (adresse du dépôt GitHub, identifiants du
compte connecté) : changements en cours enregistrés, versions en ligne récupérées (Pages CMS) en avance rapide ou
fusion sans conflit, puis versions du site envoyées en avance rapide seulement (rien n'est écrasé en ligne). En
cas de conflit rien n'est changé : les fichiers sont indiqués et l'on choisit, pour eux, la version du site ou
celle en ligne. La publication synchronise avant de publier quand l'option du profil est active (par défaut) et
s'arrête sur un conflit. Sources : [Save and restore backups — Webflow Help
Center](https://help.webflow.com/hc/en-us/articles/33961244069395), [Dealing with non-fast-forward errors —
GitHub Docs](https://docs.github.com/en/get-started/using-git/dealing-with-non-fast-forward-errors).

**D-39 — 16/10/2026 — prise.** **Canevas, suite (3.11).** Poignées de taille sur l'élément sélectionné (bord
droit, bord bas, coin ; Maj sur le coin : proportions gardées ; Échap : annulé ; double-clic : valeur retirée) :
largeur et hauteur en px, `box-sizing` respecté, écrites dans la classe réglée du panneau de styles (sinon la
première de l'élément), à l'état et au point de rupture affichés ; un élément sans classe en reçoit une d'après
sa nature (`div-block`, `heading`, `image`…, numérotée si elle existe), comme Webflow qui crée une classe quand
on règle un élément qui n'en a pas. Images (`<picture>`) : classes et tailles portées par `<img>`, désormais
réglables dans le panneau de styles. Marges (orange) et espacements intérieurs (vert) de l'élément survolé, plus
marqués quand la rubrique « Espacement » est survolée. Contours, poignées et étiquettes à taille constante quel
que soit le zoom. Sélection multiple (Maj+clic dans le canevas, Maj ou Ctrl+clic dans les calques) : supprimer,
dupliquer, copier, couper, coller et envelopper (éléments voisins) en une étape d'annulation ; le panneau règle
l'élément principal. Palette de commandes (Ctrl+K, dans toutes les vues du site ; reste « lien » dans un texte
riche) : actions, affichage, éléments à ajouter, vues et rubriques, pages, écrans, calques de la page, recherche
sans accents. Tous les écrans côte à côte (du plus large au plus étroit, ordinateur à 1280 px) : un clic dans un
écran ou sur son nom le choisit (poignées et styles y suivent), les boutons des écrans affichent ou masquent
chaque écran (au moins un reste). Cadre étirable vers le bas (poignée sous le cadre, double-clic : hauteur par
défaut) pour voir plus de la page sans la faire défiler : la mise en page ne change pas, les unités `vh`, `svh`,
`lvh`, `dvh` des feuilles et des pages servies au canevas étant rapportées à la hauteur normale du cadre
(`--rv-vh`, rendu identique tant que le cadre n'est pas allongé ; site publié inchangé) ; limite : un élément
`position: fixed` en bas de fenêtre suit le bas du cadre allongé. Page entièrement visible : la molette déplace
la vue. Sources : [Automatic class name generation — Webflow](https://webflow.com/updates/automatic-class-name-generation),
[Creating breakpoints in Framer — Framer Academy](https://www.framer.com/academy/lessons/framer-fundamentals-breakpoints).

**D-40 — 16/10/2026 — prise.** **Réglages lisibles à la place des calculs des gabarits (3.12, D-28).** Revel
reconnaît dans le document les calculs les plus utiles d'un site importé et les présente comme des réglages ;
chaque choix réécrit l'endroit exact du calcul par une commande annulable, et un calcul qui ne suit pas une forme
reconnue n'est pas proposé (il reste dans le panneau Variables, fermé par défaut). Titre de l'onglet
(Paramètres › Référencement) : forme `(seo.title ~ " | " ~ site.title) if seo and seo.title else site.title`
suivie depuis `<title>` (expression écrite dans la balise, affectation `{% set %}` ou filtre du site, nouvelle
commande `SetFilter`) : « Page | Site », « Site | Page » ou « Page seule », séparateur au choix, exemple affiché.
Image de partage par défaut et adresse officielle : clés des données du site reprises par le gabarit quand une
page n'a pas d'image (`site.share_image`) et pour les adresses canoniques (`site.url`), écrites dans le fichier
de données. Menus (Paramètres › Navigation) : listes de liens `{% set nav = [{ "url": …, "label": … }, …] %}`
(clés `url`/`href`/`link`/`path` et `label`/`title`/`name`/`text`), texte, adresse (pages proposées), ordre,
ajout, retrait ; les autres clés d'un lien le suivent. Contenus mis en avant (paramètres de la page) : listes
d'identifiants lues dans les données (`for slug in home.featured` avec `collections.x | by_slug(slug)`, ou
`collections.x | by_slugs(home.news)`, alias `{% set home = pages.accueil %}` suivis), libellé repris de Pages
CMS, contenus des collections proposés par leur titre (`data.slug`), ordre, ajout, retrait. Les listes écrites au
même retrait que leur clé (`clé:` puis `- a`, style de Pages CMS) sont réécrites entières.

**D-41 — 16/10/2026 — prise.** **Serveur MCP de Revel (4.1, D-13, D-31).** Les outils sont écrits une fois
(`revel-mcp`, SDK officiel `rmcp`) et appellent une interface `Invoke` : soit le cœur directement (application,
ligne de commande), soit l'application ouverte par HTTP local. **Un seul écrivain** : quand un assistant lance
`revel --mcp` (stdio), ce relais n'ouvre pas le site lui-même ; il transmet chaque outil à l'application ouverte
(`POST /invoke/<commande>` sur 127.0.0.1, clé lue dans `mcp.json` du dossier des réglages, aucune clé à recopier
dans la configuration de l'assistant) et la lance si elle est fermée. Les modifications passent par les mêmes
commandes que l'interface : historique commun (Ctrl+Z dans Revel annule un geste de l'assistant), enregistrement
immédiat, affichage en direct (l'éditeur relit le document quand la révision de la session du cœur change).
Le site visé est celui ouvert dans l'éditeur, ou celui nommé par l'argument `site` (sans changer celui de
l'éditeur). HTTP en flux sur `/mcp` : protocoles 2025 (session) et 2026-07-28 (sans session, en-têtes
`MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name`), réponses JSON ; écoute sur 127.0.0.1 seulement, désactivé
par défaut, clé exigée (`Authorization: Bearer`, comparaison en temps constant), requête refusée si elle porte un
en-tête `Origin` ou un `Host` qui n'est pas local (rattachement DNS), comme l'exige la spécification. Sous
Windows, l'exécutable sans console reçoit quand même les entrées et sorties fournies par l'assistant (poignées
héritées) ; l'application lancée par le relais ne les hérite pas. Descriptions des outils en anglais (langue la
mieux suivie des modèles), textes du site gardés dans leur langue. Sources :
[Streamable HTTP, spécification MCP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http),
[rmcp — docs.rs](https://docs.rs/crate/rmcp/3.1.0/source/README.md),
[GetStdHandle — Microsoft Learn](https://learn.microsoft.com/uk-ua/windows/console/getstdhandle).

**D-42 — 16/10/2026 — prise.** **Brancher un assistant (4.2).** Réglages › Assistants IA donne pour chaque
assistant la configuration exacte, avec le chemin réel du programme Revel installé, et l'endroit où la coller,
plutôt qu'un assistant pas à pas : Claude Desktop et LM Studio (même notation JSON `mcpServers` → `command`,
`args`), Claude Code (`claude mcp add --transport stdio --scope user revel -- "<programme>" --mcp`), ChatGPT de
bureau, Codex en ligne de commande et extension des éditeurs (même fichier `~/.codex/config.toml`, table
`[mcp_servers.revel]`, chemin en chaîne littérale TOML pour Windows). Tous passent par le relais stdio, sans clé
à recopier. Open WebUI ne parle MCP qu'en HTTP en flux (version 0.6.31 et plus) : type, adresse, authentification
« Bearer » et clé à reporter ; installé avec Docker, il joint l'ordinateur par `host.docker.internal`, nom désormais
accepté par le contrôle d'hôte du serveur. Les noms des menus des autres logiciels restent ceux de leur
documentation (en anglais). Les dernières actions des assistants (heure, assistant d'après le nom qu'il donne au
protocole, action, cible) sont relevées par l'application pour chaque modification réellement faite, que l'outil
s'exécute dans l'application (HTTP) ou dans le relais (assistant et outil transmis avec la commande). Sources :
[Connect to local MCP servers — modelcontextprotocol.io](https://modelcontextprotocol.io/docs/2026-07-28/develop/connect-local-servers.md),
[Model Context Protocol — Codex](https://learn.chatgpt.com/docs/extend/mcp.md?surface=cli),
[MCP in LM Studio](https://lmstudio.ai/docs/app/mcp),
[Model Context Protocol — Open WebUI](https://docs.openwebui.com/features/extensibility/mcp.md),
[Claude Code MCP setup (guide tiers)](https://mcpplaygroundonline.com/blog/claude-code-mcp-setup-best-servers-guide.md).

**D-43 — 16/10/2026 — prise.** **Mêmes actions en HTTP simple, décrites en OpenAPI (4.3).** Pour les outils qui
ne parlent pas MCP (scripts, automatisations, serveurs d'outils OpenAPI comme ceux d'Open WebUI), le serveur local
expose chaque outil en `POST /api/<outil>` avec les mêmes arguments et le même texte de résultat, et sa
description OpenAPI 3.1 en `GET /openapi.json`, générée à partir de la liste des outils (aucune seconde définition
à tenir à jour ; un essai vérifie que chaque outil du protocole y est appelable). Même serveur, même clé
(`Authorization: Bearer`), mêmes contrôles d'origine et d'hôte. Sources :
[OpenAPI Specification 3.1.0](https://spec.openapis.org/oas/v3.1.0),
[Model Context Protocol — Open WebUI](https://docs.openwebui.com/features/extensibility/mcp.md).

**D-44 — 16/10/2026 — prise (décision de l'utilisateur).** **Pas d'accès à distance (4.4 écarté).** Revel ne
s'ouvre pas aux assistants utilisés dans une page web (claude.ai ou ChatGPT dans le navigateur, applications
mobiles) : leurs serveurs ne peuvent joindre l'ordinateur que par une adresse publique, donc par un tunnel
d'un prestataire (Cloudflare, ngrok, Tailscale…), contraire au principe d'indépendance (D-36). Seuls les
assistants installés sur l'ordinateur se branchent : par la commande `revel --mcp` (Claude Desktop, Claude Code,
ChatGPT de bureau, Codex, LM Studio) ou par l'adresse locale 127.0.0.1 (Open WebUI installé chez l'utilisateur,
outils OpenAPI). Dans Claude Desktop, seul le fichier de configuration (Settings › Developer › Edit Config) donne
un branchement local ; « Add custom connector » attend une adresse publique. Le code de l'accès à distance,
écrit puis retiré le même jour (serveur d'autorisation OAuth, adresse publique, demandes d'accès), reste dans
l'historique du dépôt (commit 61d227a). Rappel : avec un modèle en ligne (Claude, ChatGPT), ce que l'assistant
lit du site part chez son fournisseur comme le reste de la conversation ; seul un modèle local garde tout sur
l'ordinateur. Sources :
[Connect to local MCP servers — modelcontextprotocol.io](https://modelcontextprotocol.io/docs/2026-07-28/develop/connect-local-servers.md),
[Add a connector that isn't in the directory — Claude](https://claude.com/docs/connectors/custom/add-unlisted),
[Model Context Protocol — Codex](https://learn.chatgpt.com/docs/extend/mcp.md?surface=cli).

**D-45 — 16/10/2026 — prise (décision de l'utilisateur).** **Pas de discussion intégrée (4.5 écartée, D-31
précisée).** On discute avec l'assistant dans son application (téléchargée par l'utilisateur), qui pilote Revel
par les outils de D-41 ; Revel n'a pas de panneau de discussion, son interface reste plus sobre.

**D-46 — 16/10/2026 — prise (décision de l'utilisateur).** **Installateur.** Le modèle de traduction
français ↔ anglais reste livré dans l'installateur jusqu'à la version 1.0 (D-36). La version 1.0 aura un
installateur plus léger et plus soigné, au logo de Revel, sans langue de traduction livrée (toutes téléchargées
depuis Réglages › Traduction automatique) ; à faire à l'étape 6 (empaquetage).

**D-47 — 16/10/2026 — prise.** **Publier en un clic (5.1) : pages modifiées calculées sur l'ordinateur.** Le
bouton « Publier » de la barre supérieure ouvre un panneau ancré (comme Webflow et Framer) : une ligne par
hébergement (adresse du site, « Site public » ou « Version de test », date de la dernière publication), cases à
cocher s'il y en a plusieurs, pages modifiées dépliables, avancement, puis « En ligne » et « Voir le site ».
Les modifications non publiées sont connues sans interroger le serveur : à chaque publication réussie, Revel
garde dans son état (`revel-state.json`) l'empreinte (BLAKE3) du HTML de chaque page générée et une empreinte des
fichiers copiés (styles, médias, scripts) ; l'état actuel est généré en mémoire et comparé (pages ajoutées,
modifiées, supprimées ; « Styles, médias et scripts » en une ligne). Le résultat est gardé tant que le site ne
change pas (révision de la session d'édition), le panneau le relit à l'ouverture et toutes les 8 s, fenêtre
visible. Limite : une publication faite hors de cette installation de Revel (autre ordinateur, outil FTP)
n'est pas connue ; la date affichée est celle de la dernière publication depuis Revel.

**D-48 — 16/10/2026 — prise (décision de l'utilisateur pour la liste).** **Première mise en ligne guidée
(5.2).** Hébergeur choisi dans une liste déroulante (OVHcloud, o2switch, Infomaniak, Hostinger, IONOS, GitHub
Pages) avec un bouton « Autre » à côté, plutôt que des tuiles. Pour chaque hébergeur, les étapes pour obtenir
les accès SFTP (activer SFTP chez OVHcloud, autoriser l'adresse IP chez o2switch, activer SSH chez Hostinger,
créer l'accès chez IONOS) et le lien de l'espace client, d'après leurs aides officielles (`docs/HEBERGEURS.md`) ;
port (65002 chez Hostinger) et dossier habituel préremplis. Le courriel d'installation ou la page des accès,
collés tels quels, sont lus sur l'ordinateur (`revel_protocol::hosting::parse_access` : libellés français et
anglais, valeur à la ligne, adresse `sftp://`) ; le message est effacé ensuite car il contient le mot de passe.
o2switch n'a d'aide qu'en français : ses libellés restent en français dans l'interface anglaise.

**D-49 — 16/10/2026 — prise.** **Connexion d'essai et clé du serveur (5.3).** « Se connecter » ouvre une session
SFTP sans rien écrire (`revel_deploy::sftp::probe`, délai de 20 s) : accès vérifiés, clé du serveur enregistrée
avec l'hébergement sans l'afficher (confiance à la première connexion, comme `ssh` avec
`StrictHostKeyChecking=accept-new`), dossiers du site trouvés (`domains/<domaine>/public_html`,
`sites/<domaine>`, `www`, `public_html`, `htdocs`, `httpdocs`, `html`, `web`) ; dossier proposé : celui du
domaine du site, sinon le dossier habituel de l'hébergeur, sinon le premier trouvé. La clé n'est montrée qu'en
cas de changement (échec « clé du serveur changée »). Sans réponse (pare-feu d'o2switch sans adresse IP
autorisée), l'échec le dit.

**D-50 — 16/10/2026 — prise.** **Contrôles avant publication (5.4).** Faits sur le site généré (ce qui sera mis
en ligne) : liens internes vers une page ou un fichier absent, images de plus de 1 Mo envoyées telles quelles,
images sans attribut `alt` (un `alt=""` volontaire est accepté), pages sans `<title>` ; pages de redirection
ignorées ; liens externes non vérifiés (pas de requête vers d'autres sites). Une publication s'arrête et le
panneau montre les points regroupés ; « Publier quand même » relance sans contrôle. Plusieurs hébergements :
contrôles une seule fois.

**D-51 — 16/10/2026 — prise.** **Revenir à la publication précédente (5.5).** Chaque publication réussie est
gardée dans le dossier du site (`.revel/published`) : fichiers rangés par empreinte BLAKE3 (un contenu identique
n'est gardé qu'une fois d'une version à l'autre), liste des fichiers des deux dernières versions par
hébergement, fichiers inutilisés retirés. Revenir republie la version précédente par le même envoi qu'une
publication (seuls les fichiers qui diffèrent partent, SFTP comme GitHub Pages) ; la version remplacée devient
la précédente, un second retour annule le premier. Les fichiers sont copiés (pas de liens physiques vers le
dossier généré, réécrit à chaque génération).

**D-52 — 16/10/2026 — prise.** **GitHub Pages en un clic (5.6).** Dépôt proposé `compte/nom-du-site` ; s'il
n'existe pas au nom du compte connecté, il est créé public (GitHub Pages gratuit) sans commit initial à la
première publication (`POST /user/repos`), puis la branche `gh-pages` est envoyée et GitHub Pages activé comme
avant. Pas de création pour un dépôt d'organisation ni pendant une vérification. Source :
[Create a repository for the authenticated user](https://docs.github.com/en/rest/repos/repos#create-a-repository-for-the-authenticated-user).

**D-53 — 16/10/2026 — prise.** **Nom de domaine (5.7).** Pour GitHub Pages : quatre enregistrements A, quatre AAAA
(facultatifs) et CNAME `www` vers `compte.github.io`, prêts à copier
([documentation GitHub](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site)) ;
pour les autres hébergeurs, chemin de leur espace client (leurs adresses IP dépendent du serveur). Vérification
par Revel, sans service extérieur : adresses données par le résolveur du système, requête HTTPS (certificat
vérifié) puis HTTP vers le domaine, page d'accueil reçue comparée à l'empreinte du fichier `index.html` de la
dernière publication. Le domaine enregistré devient l'adresse du site publié (« Voir le site ») et, pour GitHub
Pages, le domaine personnalisé envoyé à la publication suivante.

**D-54 — 16/10/2026 — prise.** **Formulaires (5.8).** Les `<form data-revel-form="nom">` (bloc « Formulaire »
du panneau Ajouter) envoient leurs champs à `/revel-form.php`, produit à la génération quand une adresse de
réception est réglée (Paramètres › Formulaires) : fonction `mail()` de PHP de l'hébergement, expéditeur
`site@<domaine du site>` (exigé par OVHcloud et IONOS), visiteur en `Reply-To`, champ piège, dix envois par
heure et par adresse IP, réponse JSON (envoi sans recharger la page) ou page de confirmation. Compatible PHP 7.0.
GitHub Pages n'exécute pas PHP : signalé dans Paramètres › Formulaires et par l'avertissement de publication.
Les formulaires qui ont déjà leur propre `action` (comme `contact.php` du site Alunites) ne sont pas touchés.
Limites des hébergeurs : `docs/HEBERGEURS.md` (Infomaniak : fonction mail à activer, 50 messages par 24 h).
