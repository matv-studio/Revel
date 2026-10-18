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

**D-58 — 16/10/2026 — prise (espacements : remplacé par D-60).** **Canevas : réglages directs.** Marges et espacements intérieurs survolés en
hachures dans la couleur de sélection de Revel ; sur l'élément sélectionné, un côté se règle en le glissant
(valeur affichée, Maj : les quatre côtés, Alt : le côté opposé, double-clic : valeur retirée), comme Framer.
Largeur du cadre réglable par des poignées sur ses bords (responsive horizontal, point de rupture suivi,
double-clic : largeur du point de rupture). Ctrl+Espace ouvre la palette de commandes comme Ctrl+K (sur macOS, Contrôle-Espace
sélectionne la source de saisie précédente quand plusieurs sont installées et n'atteint alors pas Revel :
[Apple](https://support.apple.com/guide/mac-help/mchlp1406) ; Ctrl+K reste). Un
texte lié à une donnée dans un composant (titre du site, lien du menu) se modifie au double-clic ; un texte fixe
d'un composant ouvre le composant et passe en modification du texte au même double-clic.

**D-59 — 16/10/2026 — prise.** **Animations prêtes à l'emploi (6.1), à l'image des effets de Framer.** Six
moments (apparition, texte, survol, appui, boucle, défilement), 44 animations, quelques réglages chacune
(catalogue : `docs/ETAPE-6.md`). Un attribut par moment sur l'élément, `data-revel-appear="up dur=0.8 replay"`
(l'animation puis les réglages qui diffèrent des valeurs par défaut) : lisible dans le code exporté, sans
changement du modèle, modifié par les commandes d'édition existantes (annulation, assistants). Script du site
`/assets/revel-animations.js` (19 Ko, 6 Ko compressé), produit seulement si une page est animée : Web
Animations, transformations et filtres ajoutés à ceux de l'élément (`composite: "add"`, disponible partout
depuis 2022, [MDN](https://developer.mozilla.org/en-US/docs/Web/API/KeyframeEffect/composite)),
IntersectionObserver pour l'arrivée à l'écran et la pause des boucles hors de l'écran. Animations CSS liées au
défilement (`animation-timeline`) écartées : pas encore disponibles dans tous les navigateurs
([MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timeline)). Contenu visible sans JavaScript
et si le script ne se charge pas (3 s) ; rien n'est animé pour `prefers-reduced-motion`. Texte découpé en
lettres ou mots au chargement de la page (pas dans la source), texte complet gardé pour les lecteurs d'écran.
Images et composants animés par un bloc qui les entoure (« Envelopper pour animer »). Aperçu : le même script,
chargé sans démarrage automatique, joue l'animation dans le canevas et sur les vignettes du panneau.

**D-60 — 16/10/2026 — prise.** **Espacements dans le canevas : poignées (remplace les hachures de D-58).** Quand
le pointeur est au-dessus de l'élément sélectionné (marges comprises), ses espacements intérieurs et les écarts
entre ses enfants (conteneur flex ou grille) apparaissent en teinte unie de la couleur de sélection, ses marges
non nulles en teinte plus claire, sans hachures ; une poignée (pilule blanche bordée) au milieu de chaque côté,
dans la partie visible de la page pour un élément plus haut que l'écran. Seule la poignée se glisse (valeur
affichée ; Maj : les quatre côtés, Alt : le côté opposé, comme dans le panneau) ; un clic sur la poignée sans
glisser met le champ correspondant du panneau de styles en saisie. Repères : poignées d'espacement de Figma
(Alt : côtés opposés, [aide Figma](https://help.figma.com/hc/en-us/articles/360040451373)) et de Framer
(intérieur et écarts réglés sur le canevas).

**D-61 — 16/10/2026 — prise.** **Calques lisibles sans connaître le code.** Une boucle s'appelle « Liste · »
suivi du dernier mot de sa source (`home.featured` → « Liste · Featured »), son expression en info-bulle ; une
condition sans « sinon » n'a pas de ligne : ses éléments portent une marque de condition (comme la visibilité
conditionnelle de Webflow) ; une condition avec « sinon » s'appelle « Condition · » suivi de la donnée testée.
Boucles et conditions qui ne font que préparer des données (`{% set %}` seuls) absentes. Liens d'évitement et
textes réservés aux lecteurs d'écran (`skip-link`, `sr-only`, `visually-hidden`…) absents : invisibles sur la
page, ils restent dans le site. Un texte venu des données s'affiche tel que rendu dans la page (envoyé par le
canevas), jamais son expression `{{ … }}`, avec une marque de liaison ; un bloc qui ne contient que du texte a
l'icône de texte ; double-clic sur sa ligne : texte modifié dans la page. Composants sans nom : fin de
l'identifiant rendue lisible ; images : nom du fichier, ou marque de liaison.

**D-62 — 16/10/2026 — prise.** **Canevas sans rechargement pour les attributs ; scripts du site jamais dans le
canevas.** Une classe, une animation, un lien ou tout attribut écrit (sans expression) est posé directement sur
les éléments de la page affichée (message `attrs`) : la page n'est pas rechargée, les animations ne rejouent
pas. Le bouton « scripts du site » est retiré : le canevas n'exécute jamais les scripts du site (sélection,
modification et mesures fiables), l'aperçu les exécute toujours.

**D-63 — 16/10/2026 — prise.** **Netteté du canevas dézoomé.** Position des cadres arrondie au pixel de l'écran,
`will-change: transform` seulement pendant un zoom puis retiré 200 ms après, pour que le moteur redessine le
texte à la taille affichée au lieu d'agrandir ou réduire une image figée (cause décrite dans
[Monoka, PR #49](https://github.com/monoka-ai/monoka/pull/49)). Vérifié dans Chromium sans écran ; à confirmer
dans WebView2 (Windows) et WKWebView (macOS).

**D-64 — 16/10/2026 — prise.** **Ordre d'affichage des pages.** Les pages, dossiers et collections se rangent
en les glissant dans le panneau Pages, à l'intérieur de leur dossier seulement : l'ordre est enregistré dans le
rangement de l'éditeur (`library.pages.order`, clés `page:`, `folder:`, `collection:`), sans effet sur les
adresses ni sur le site publié ; les éléments non rangés suivent dans l'ordre habituel.

**D-65 — 16/10/2026 — prise.** **Réglages de l'application sur une page.** Général (langue, thème, accent et son
étendue, dossier des projets), comptes, langues de traduction et assistants IA en cartes sur deux colonnes, sans
navigation par rubriques.

**D-66 — 16/10/2026 — prise.** **Sélection depuis les calques avec plusieurs écrans.** L'élément est sélectionné
dans tous les écrans, montré dans l'écran actif (Ordinateur par défaut, sinon l'écran en cours de
modification) : la page défile d'abord dans la partie visible du cadre, la vue ne se déplace que si l'élément
reste hors de la zone. Le canevas ne fait plus défiler l'éditeur (`scrollIntoView` remplacé par un défilement
de la page seule, sans l'animation `scroll-behavior: smooth` du site ; zone du canevas en `overflow: clip`).

**D-67 — 16/10/2026 — prise.** **Aperçu intégré.** La politique de sécurité de l'interface autorise le
protocole de l'aperçu dans les cadres (`frame-src revel-preview: http://revel-preview.localhost`) ; sans elle,
WebView2 bloquait l'aperçu intégré (« Ce contenu a été bloqué »), l'aperçu en fenêtre restant possible
([MDN, frame-src](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Content-Security-Policy/frame-src)).

**D-68 — 16/10/2026 — prise.** **Canevas des composants et variantes, comme Framer.** « Modifier le composant »
(réglages d'une utilisation, panneau Composants, double-clic dans une utilisation ailleurs que sur un texte
écrit) ouvre le composant seul dans le canevas : un cadre par variante (principale, survol, appui,
personnalisées), à la largeur de l'écran choisi, côte à côte pour un composant étroit, les uns sous les autres
sinon ; « + » ajoute une variante, clic droit sur son nom pour la renommer ou la retirer ; une barre en bas
ramène à la page et règle la transition. Les variantes sont décrites dans le composant (`variants`) et leurs
styles sont des règles ordinaires de la feuille du site, préfixées par la classe de l'élément racine
(`.carte:hover .titre`, `.carte:active`, `.carte.is-sombre .titre`) : rien n'est produit en JavaScript, le site
publié reste du HTML et du CSS. L'élément racine reçoit une classe s'il n'en a pas (d'après le nom du
composant). Dans le cadre d'une variante, le panneau de styles écrit pour elle (pastille verte) et montre en
orange les valeurs héritées de la principale. Les cadres de survol et d'appui montrent l'état sans le pointeur :
leurs règles `:hover` / `:active` sont recopiées sur une classe posée sur la racine (canevas seulement). Une
utilisation choisit sa variante personnalisée (classe `is-<id>` posée sur la racine au rendu). Transition des
variantes : règle `.racine, .racine *` (propriétés visuelles, 0,15, 0,3 ou 0,6 s, courbe douce). Passage
d'une variante à l'autre au clic : interactions entre éléments (6.3). Repères :
[Framer, variantes de composant](https://www.framer.com/academy/lessons/component-variants-in-framer),
[Webflow, canevas des composants](https://help.webflow.com/hc/en-us/articles/49505240420755-Component-canvas),
[Webflow, variantes](https://help.webflow.com/hc/en-us/articles/51307110086547-Component-variants).

**D-69 — 16/10/2026 — prise.** **Animer un composant sans l'envelopper ; onglet Réglages fondu dans Styles.**
Une utilisation de composant porte ses propres attributs (`attrs`), posés sur l'élément racine du composant au
rendu : le panneau Interactions anime une utilisation directement (seule une image demande encore un bloc
autour). L'onglet Réglages disparaît (deux boutons « Réglages » et « Paramètres » faisaient des choses
différentes) : en tête de l'onglet Styles, le contenu de l'élément (textes, lien et nouvel onglet, image ou
média, utilisation de composant avec sa variante et ses propriétés, condition, liste, réglages d'un bloc) ; en
bas, la rubrique repliée « Paramètres avancés » (balise, identifiant, autres attributs).

**D-70 — 16/10/2026 — prise.** **Canevas libre ou vue verrouillée, comme Webflow.** Un bouton de la barre de
l'éditeur (et la palette de commandes) bascule entre le canevas libre (déplacer la vue, zoomer, régler la
largeur et la hauteur du cadre, plusieurs écrans côte à côte) et la vue verrouillée : page calée sur la zone,
zoom ajusté, la molette fait défiler la page où que soit le pointeur, ni déplacement ni zoom au pincement,
poignées du cadre masquées. Choisir un zoom ou les écrans côte à côte repasse en canevas libre ; le canevas
des composants reste toujours libre (ses variantes dépassent la zone). Choix gardé d'une session à l'autre ;
canevas libre par défaut (comportement des versions précédentes). Repère :
[Webflow, pan and zoom for pages](https://webflow.com/updates/page-pan-zoom) (« The canvas scrolls top to bottom
by default » ; une icône de la barre du canevas active déplacement, zoom et largeur du cadre).

**D-71 — 16/10/2026 — prise.** **Couleurs du site et paramètres regroupés.** Dans le sélecteur de couleur, « + »
ajoute la couleur choisie aux couleurs du site : nom demandé, variable `--nom` déclarée dans `:root`, la
propriété réglée la reprend (comme les styles de couleur de Framer et les variables de Webflow). Les champs de
taille montrent une variable du site par son nom (« Step 3 » pour `var(--step-3)`) et l'acceptent écrite ainsi.
Paramètres du site : six rubriques au lieu de dix (Général avec référencement et formulaires, Publication avec
export, Navigation avec redirections, Polices, Code personnalisé, Versions), parties côte à côte sur un grand
écran.

**D-72 — 16/10/2026 — prise.** **Mise en forme dans le texte.** Pendant la modification d'un texte sur le
canevas, une barre apparaît au-dessus de la sélection : gras, italique, lien, span (texte à styler à part), comme
la barre de texte de Webflow. Ctrl+B, Ctrl+I et Ctrl+U font de même. La mise en forme devient des éléments
`strong`, `em`, `u`, `a` et `span` dans le document (une seule annulation pour la saisie) ; tout autre balise
collée est réduite à son texte.
- Complément du 17/10/2026 (recréation du site Alunites) : un élément qui ne contient que du texte et des éléments
  de mise en forme (titre avec des spans, paragraphe avec du gras ou des retours à la ligne) se modifie **d'un
  bloc**, sur la page comme dans le champ « Texte » du panneau. Les éléments déjà présents sont repris par leur
  repère (classes, animations et autres attributs gardés) ; une copie collée reçoit un nouvel identifiant ; une
  seule commande remplace le contenu de l'élément. Le gras, l'italique et le soulignement sont posés par Revel
  (le navigateur poserait un « gras retiré » en style dans un titre déjà gras). Le double-clic sélectionne le mot
  pointé. Hors de la langue principale, les textes restent traduits un par un.

**D-73 — 17/10/2026 — prise.** **Réglages ajoutés après la recréation du site Alunites** (accueil et modèle d'un
projet, refaits depuis l'interface seule) : ce qui obligeait à un détour devient un réglage.
- Grille : colonnes **égales** (nombre), **sur mesure** (largeur de chaque colonne : `2fr`, `240px`, `auto`) ou
  **automatiques** (autant que la place permet, largeur minimale, étirées ou non) ; écart des lignes et des
  colonnes séparé par un cadenas (lu et écrit dans `gap` quand il y est) ; enfant d'une grille : cases
  occupées, alignement propre, ordre ; enfant d'une pile : alignement propre, ordre.
- Effets : mélange (`mix-blend-mode`), filtres et filtres de l'arrière-plan (flou, luminosité, contraste, noir et
  blanc, saturation, sépia, inversion, teinte), transformations (déplacement, rotation, échelle, inclinaison),
  transitions (ce qui change, durée, délai, courbe) ; une forme écrite à la main que Revel ne sait pas lire
  reste modifiable en texte.
- Texte : retour à la ligne (automatique, jamais, retours écrits gardés) et coupure des lignes (équilibrées, sans
  mot seul à la fin).
- Contenus (CMS) : écriture d'une date reliée choisie dans une liste (16.10.2026, 16 octobre 2026, octobre
  2026…) ; les mois sont écrits dans la langue de la page (`date_text(locale, "long")`, filtre du moteur) ; texte
  écrit avant et après une valeur reliée (« pour Volvo Trucks ») ; affichage d'un élément selon un champ rempli
  ou vide (condition qui l'enveloppe, lisible aussi quand elle est sélectionnée) ; image de fond d'un bloc
  reliée à un champ ; liste « sans le contenu affiché » sur la page d'un contenu (`without(page.url)`) ; liste
  dans une liste tirée d'un champ à plusieurs valeurs (valeurs, ou contenus désignés par une référence) ; listes
  nommées dans les calques d'après la collection ; à la création d'une collection, dossiers de contenus qu'aucune
  collection ne lit proposés (champs déduits des contenus).
- Une utilisation de composant dans une liste ne montre plus dans son champ « Texte » les textes du contenu de
  la liste (ils se modifient dans le CMS).

**D-74 — 17/10/2026 — prise.** **Animations Lottie (6.2) : lecteur lottie-web hébergé avec le site.** Bloc
« Animation Lottie » du panneau Ajouter ; fichier choisi parmi les médias (fichier `.json` exporté d'After Effects
par Bodymovin, ou archive `.lottie` de LottieFiles dont l'animation est extraite en `.json` à l'import) ; lecture à
l'écran (pause hors de l'écran), au survol (avance puis revient), à l'appui ou au défilement ; boucle ; vitesse.
- Lecteur : lottie-web 5.13.0 (Airbnb, licence MIT, `crates/revel-render/vendor/lottie-web/`), rendu SVG avec
  expressions (`lottie_svg.min.js`, 243 Ko, environ 60 Ko compressé), plutôt que la version légère sans
  expressions (168 Ko) : les animations d'After Effects qui en utilisent restent lues. Produit en
  `/assets/revel-lottie.js` (lecteur et réglages) seulement si une page contient une animation ; aucun service
  extérieur (D-36). Le canevas reçoit le même script : l'animation s'y voit et suit ses réglages sans recharger
  la page.
- Mouvement réduit demandé par le visiteur : pas de lecture automatique. Sans hauteur donnée par le style, la
  proportion de l'animation est appliquée.
- Un fichier `.json` des médias n'est rangé parmi les animations que s'il en a la forme (cadence, début, fin,
  calques) ; un autre JSON est refusé à l'import des médias.

**D-75 — 17/10/2026 — prise.** **Interactions entre éléments (6.3), sans code.** Rubrique « Actions » du panneau
Interactions : déclencheur (à l'appui, au survol, au défilement au-delà de N pixels, à l'écran, au chargement),
action (ajouter une classe, afficher ou masquer, afficher, masquer, lire ou mettre en pause une vidéo, fermer
pour de bon), élément visé (cet élément, parent, suivant, page entière, éléments d'une classe, élément nommé).
- Écrites dans l'attribut `data-revel-on` de l'élément déclencheur (`click class:is-open #menu; scroll:80
  class:is-solid self`), lisible et modifiable à la main ; une forme non reconnue reste modifiable en texte.
  Script `/assets/revel-actions.js` (quelques Ko) produit si une page en contient ; pas dans le canevas (un
  clic y sélectionne), mais dans l'aperçu.
- Couvre les comportements du site Alunites : menu ouvert par un bouton (classe sur la navigation), en-tête
  opaque après le défilement, vidéo mise en pause, bandeau de consentement fermé pour de bon (souvenu par le
  navigateur du visiteur).
- Onglets prêts à l'emploi (panneau Ajouter) : un bouton par panneau, le premier ouvert ; sans JavaScript, tous
  les panneaux restent lisibles. Accordéon prêt à l'emploi : élément `details` du navigateur, sans script.

**D-76 — 17/10/2026 — prise.** **Transitions entre les pages (6.4) par les transitions de vue du navigateur.**
Paramètres › Général : aucune, fondu ou glissement. Écrites en CSS dans chaque page (`@view-transition {
navigation: auto }`), sans script : prises en charge par Chrome et Edge 126 et Safari 18.2 ; Firefox change de
page sans transition (vérifié le 17/10/2026, https://developer.chrome.com/docs/web-platform/view-transitions).
Coupées pour les visiteurs qui demandent moins de mouvement ; absentes du canevas.

**D-77 — 17/10/2026 — prise.** **Mises à jour de Revel (6.6) : module de mise à jour de Tauri, versions publiées
sur GitHub.** Revel cherche au lancement une version plus récente (`latest.json` de la dernière version publiée
du dépôt `matv-studio/revel`) ; un bouton « Version x.y.z » apparaît alors dans la barre supérieure, et
Réglages › Mises à jour montre la version installée, cherche, installe et relance (installation « passive »
sous Windows).
- Chaque mise à jour est vérifiée par sa signature avant d'être installée (clé publique dans la configuration,
  clé privée seulement dans les secrets du dépôt). Une version construite sans clé (développement) ne cherche
  rien et le dit.
- À faire par l'utilisateur pour activer les mises à jour (réglages persistants du dépôt, non faits par
  Claude) : créer la paire de clés sur son ordinateur (`cargo tauri signer generate -w ~/.tauri/revel.key`),
  déclarer le secret `TAURI_SIGNING_PRIVATE_KEY` (contenu de la clé privée) et, s'il y en a un,
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, puis la variable `TAURI_UPDATER_PUBKEY` (contenu de la clé publique).
  Perdre la clé privée empêche de publier des mises à jour pour les versions déjà installées. Une version
  n'est proposée qu'une fois publiée (pas en brouillon ni en préversion).
- 17/10/2026 : clés déclarées par l'utilisateur ; la 0.6.0 est la première version construite avec elles
  (`latest.json` et une signature par installateur, 11 cibles ; signature du `.deb` vérifiée avec la clé
  publique lue dans l'application construite). Les adresses de `latest.json` sont celles de l'API GitHub
  (`/releases/assets/<id>`), que le module de mise à jour télécharge avec l'en-tête
  `Accept: application/octet-stream` (tauri-plugin-updater 2.14.0). Le flux crée des brouillons marqués
  « préversion » : en publier un comme mise à jour demande de décocher « Set as a pre-release ». Les versions
  0.5.0 et antérieures n'ont pas de clé et ne se mettent pas à jour seules.

**D-78 — 17/10/2026 — prise.** **Retours sur la 0.5.0 : canevas mis à jour sur place, disposition à la Webflow,
ajout au double-clic.**
- Le canevas n'est plus rechargé après une modification : la page rendue est relue et comparée à celle
  affichée, seuls les nœuds qui diffèrent sont remplacés (les animations déjà jouées ne repartent pas). Le rendu
  porte le numéro de révision de la session (`data-rev`), pour reconnaître un canevas en retard. Le
  rechargement complet reste le recours si l'en-tête de la page change (feuilles, scripts) ou si la mise à jour
  échoue.
- Panneau Ajouter : un clic sur une tuile n'ajoute rien, un double-clic ajoute, le glisser-déposer reste. Un
  conteneur vide (boîte, section, grille, flex, liste, Lottie) est montré dans le canevas par son icône sur une
  zone d'au moins 50 px, sans changer le site produit.
- Calques : un composant n'est plus déplié, il se modifie dans son propre espace, ouvert par un double-clic.
- Disposition (panneau Styles) reprise de Webflow : affichage Bloc, Flex, Grille, Masqué et une flèche pour les
  autres ; alignement par une grille de 3 × 3 et deux lignes X et Y ; écarts de colonnes et de rangées liés ou
  non ; grille réglée par nombre de colonnes et de rangées (pistes détaillées sur demande) ; rubrique « Enfant
  de grille » ou « Enfant flex » pour l'élément placé dedans.
- Listes déroulantes à la charte par `appearance: base-select` (Chromium 135 et plus, donc WebView2 ; ailleurs
  la liste du système, aux couleurs de la charte quand c'est possible ; https://developer.chrome.com/blog/a-customizable-select).
  Barres de défilement fines, dessinées comme la piste des curseurs.
- Accueil : un site se supprime en l'envoyant à la corbeille du système (crate `trash`), après confirmation ;
  seul un dossier contenant `site.revel.json` est accepté.
- Réglages : le menu du site reste dans la barre supérieure, avec « Revenir au site » ; seules les langues
  installées sont listées, les autres sous « Plus de langues ».

**D-79 — 17/10/2026 — prise (décision de l'utilisateur).** **Aucune langue de traduction livrée, dès la
0.5.1.** Remplace D-46 sur ce point : toutes les versions construites à partir de maintenant sont sans modèle
de traduction (ni français ↔ anglais), toutes les langues se téléchargent depuis Réglages › Traduction
automatique. Installateurs plus légers (le modèle français pesait environ 300 Mo). Les langues installées sont
listées directement, « Aucune langue installée » sinon ; les autres sous « Plus de langues ».
- Une installation de la 0.5.0 perd le français livré à la mise à jour (les ressources de l'application sont
  remplacées) : il se retélécharge depuis les Réglages.
- Construction : étape « modeles » retirée du flux des installateurs, ressource `models/` retirée de
  `tauri.conf.json`.

**D-80 — 17/10/2026 — prise (option 1 choisie par l'utilisateur).** **Import depuis Git : sites statiques et
Eleventy seulement, sans construction.** Un dépôt est copié (libgit2, sans Git installé) dans le dossier des
projets, puis importé comme un dossier : HTML statique (7.1) ou projet Eleventy. Un site à construire
(application JavaScript : SvelteKit, Next, Nuxt, Astro, Vite, Gatsby…) n'est pas construit par Revel : il
faudrait Node ou Bun sur l'ordinateur (contraire à D-36), et le résultat serait une application dont le contenu
est produit par JavaScript, non modifiable dans l'éditeur. Revel le signale et propose d'importer le dossier
produit par sa construction (`dist`, `build`, `out`, `_site`, `public`) s'il est dans le dépôt.
- L'historique du dépôt est gardé (panneau Versions) et son adresse d'origine aussi : rien n'est envoyé au
  dépôt sans « Synchroniser ». Dépôt privé sur github.com : jeton de la connexion GitHub (D-11).
- Licence : un dépôt sans licence reste la propriété de son auteur ; l'import ne vérifie pas les droits de
  reprise (avertissement quand aucun fichier `LICENSE` n'est trouvé).
- Exemple étudié : `deltea/p3r-pause-menu` (SvelteKit, adaptateur Vercel, sans licence affichée) est un site à
  construire, non importable par cette voie.
- Révisée par D-88 (18/10/2026, demande de l'utilisateur) : la construction est proposée, jamais automatique ;
  Revel télécharge alors lui-même Node.js ou Hugo (vérifiés) dans ses données, sans installation sur le système.

**D-81 — 17/10/2026 — prise.** **Import depuis un lien (7.4) : le site publié est lu, pas l'outil.** Pages
suivies depuis l'accueil, les liens et le plan du site (`sitemap.xml`), règles de `robots.txt` respectées
(`Crawl-delay` : pages lues une à une, pause plafonnée à 2 s), 2 000 pages et 2 Go au plus, six téléchargements
simultanés, interruption possible. Images, polices, scripts et
feuilles des serveurs de l'outil (framerusercontent.com, website-files.com, fonts.gstatic.com…) copiés dans le
site (`/_externe/<serveur>/…`) : le site importé ne dépend plus d'eux (D-36), connexions préparées
(`preconnect`) retirées. Les styles écrits dans les pages (`<style>`, cas de Framer) sont réunis, règles en double
fusionnées, dans la feuille du site (`/assets/styles.css`), donc modifiables dans le panneau Styles.
- Seule voie pour Framer (pas d'export HTML, voir ETAPE-7) ; marche pour tout site en ligne dont les pages sont
  écrites en HTML (pas pour une application dont le contenu est produit par JavaScript).
- Droits : l'import ne vérifie pas que l'utilisateur peut reprendre le site lu (même règle que D-80).

**D-82 — 17/10/2026 — prise.** **Moteurs des outils retirés, animations reprises (7.7).** Framer : scripts de
son moteur retirés (le HTML publié est complet ; gardés, ils recréeraient la page et effaceraient les
modifications faites dans Revel), apparitions (`framer/appear`) reprises en animations Revel (`data-revel-appear`,
modifiables dans le panneau Interactions), état de départ masqué retiré. Menus, carrousels et effets au survol
produits par ce moteur ne sont pas repris (signalé). Webflow : `webflow.js` gardé ; interactions d'apparition
d'un élément (effets prêts à l'emploi et animations sur mesure : opacité, déplacement en pixels, échelle,
rotation) reprises et détachées de Webflow ; interactions sur une classe, au défilement continu, au survol ou au
clic gardées pour Webflow. AOS (`data-aos`) repris. Formulaires Webflow envoyés par Revel (D-54).

**D-83 — 17/10/2026 — prise.** **Contenus du CMS de Webflow (7.3) : fichiers CSV ou API, jeton jamais gardé.**
Les fichiers CSV exportés (une collection par fichier, trouvés à la racine de l'export ou ajoutés) donnent des
champs déduits des valeurs (texte, texte riche, image, date, oui/non, nombre, lien, références entre fichiers) ;
l'API de Webflow (jeton du site, lecture du CMS, identifiant du site lu dans `data-wf-site`) donne les types
exacts et les brouillons. Contenus en fichiers Markdown (`content/<collection>`, décrits dans `.pages.yml`),
images copiées dans `images/cms/`. La page modèle exportée (`detail_<collection>.html`) devient le gabarit des
pages de la collection (`/<collection>/<slug>/`), éléments liés reliés aux champs dans l'ordre (à vérifier,
signalé). Les listes du CMS sont exportées vides par Webflow : à relier dans Revel (signalé).

**D-84 — 17/10/2026 — prise.** **WordPress (7.6) : fichier d'export, modèle Blog de Revel, adresses gardées.** Le
thème (PHP) n'est pas dans le fichier d'export : le site part du modèle Blog de Revel (gabarits, styles). Articles
en collection `articles` (texte HTML, image mise en avant, catégories, étiquettes, auteur, brouillons), adresse
d'origine gardée par article (`permalink`) : les liens existants restent valides, un nouvel article prend
`/articles/<slug>/`. Pages à leur adresse, premier menu de navigation repris. Images utilisées téléchargées depuis
le site (mêmes adresses `/wp-content/uploads/…`) si l'option est gardée. Codes courts d'extensions et
commentaires non repris (signalés).

**D-85 — 17/10/2026 — prise.** **Figma (7.5) : API de Figma, jeton personnel gardé dans le trousseau.** Le fichier
est lu par l'API REST (`GET /v1/files/:clé`, droit `file_content:read`), cadres de premier niveau proposés par page
du fichier ; chaque cadre choisi devient une page (le premier : l'accueil). Mise en page automatique → flex,
largeur « remplir » ou égale au parent → étirée, largeur fixe → plafonnée à l'écran ; cadres sans mise en page
automatique : positions absolues (signalé). Une classe par élément, réutilisée par les éléments du même nom aux
mêmes styles. Styles de couleur du fichier → couleurs du site (variables CSS de `:root`) ; les variables de
Figma ne sont lisibles par l'API qu'avec une place complète d'une organisation Enterprise
([documentation](https://developers.figma.com/docs/rest-api/variables/)) : non lues. Composants utilisés
plusieurs fois avec la même structure → composants Revel, textes en propriétés. Formes et icônes exportées en SVG, images téléchargées, polices ajoutées depuis Google Fonts et
copiées dans le site (D-24). Le jeton (Figma › Settings › Security) est gardé dans le trousseau du système, comme
celui de GitHub, et peut être oublié depuis la boîte d'import.

**D-86 — 17/10/2026 — prise.** **Préversions dans les mises à jour (6.6) : réglage « Recevoir les préversions ».**
L'adresse inscrite dans l'application (`…/releases/latest/download/latest.json`) ne mène qu'à la version marquée
« Latest », qu'une préversion ne peut pas recevoir (« Drafts and prereleases cannot be set as latest »,
[API de GitHub](https://docs.github.com/en/rest/releases/releases)). Réglages › Mises à jour › « Recevoir les
préversions » (réglage de l'application, désactivé par défaut) : Revel lit la liste publique des versions du même
dépôt (`api.github.com/repos/<compte>/<dépôt>/releases`, sans compte, 60 lectures par heure et par adresse IP),
retient le numéro de version le plus élevé parmi celles qui portent un `latest.json` (brouillons jamais visibles)
et cherche la mise à jour à cette adresse ; sans réponse, l'adresse inscrite sert seule. Signature vérifiée de la
même façon. Version proposée affichée « Préversion x.y.z » (barre supérieure, réglages) ; changer le réglage
relance la recherche. Retirer le réglage ne fait jamais revenir à une version plus ancienne. Rien d'autre à
publier ni à héberger : le flux des installateurs reste le même. Choix de l'utilisateur : réglage livré dans le
brouillon 0.6.0 reconstruit, sans nouveau numéro de version (commit 6c00d74, flux 39066418973 : les 17 fichiers
du brouillon remplacés, signature vérifiée avec la clé intégrée). La session ne peut pas modifier une version
GitHub (refus 403) : le brouillon vise encore 21dd2c8 ; à la publication, choisir comme cible le commit
construit, 6c00d74. Publiée en préversion le 17/10/2026 sur `main` alors à b5b7a30 (même code que 6c00d74, seuls
des documents en plus) : étiquette `v0.6.0` conforme aux installateurs.

**D-87 — 17/10/2026 — annulée le jour même.** **Un installateur par système à partir de la version qui suit la
0.6.0 : plus de `.msi` ni de `.rpm`.** Demande de l'utilisateur (17 fichiers dans le brouillon 0.6.0, dont deux installateurs de
remplacement). `bundle.targets` de `tauri.conf.json` passe de `"all"` à `["nsis", "app", "dmg", "deb",
"appimage"]` (valeurs de la [configuration de Tauri](https://v2.tauri.app/reference/config/) ; `app` donne le
`.app.tar.gz` des mises à jour macOS). Fichiers d'une version : `.exe`, `.dmg` ×2, `.app.tar.gz` ×2, `.deb`,
`.AppImage`, une `.sig` par fichier installable par mise à jour (5) et `latest.json`, soit 13. Contrepartie :
pas d'installateur MSI pour un déploiement géré par un service informatique (Intune, stratégies de groupe) ;
Fedora et openSUSE passent par l'AppImage. Le brouillon 0.6.0 garde ses 17 fichiers ; une installation faite
avec son `.msi` ou son `.rpm` ne trouverait plus de mise à jour du même type (le module cherche
`<système>-<architecture>-<installateur>`, puis `<système>-<architecture>`) : ne pas diffuser ces deux fichiers.
- Annulée (17/10/2026) : la 0.6.0 a été publiée (préversion) avec son `.msi` et son `.rpm` ; l'utilisateur garde
  les installateurs de remplacement. `bundle.targets` revient à `"all"`, pour que les installations faites avec
  ces fichiers continuent de recevoir les mises à jour de leur type.

**D-88 — 18/10/2026 — prise (trois options demandées par l'utilisateur).** **Site à construire : Revel le
construit sur demande, le site publié s'importe, le lien garde les scripts du site.** Demande du 17/10/2026 après
l'import bloqué du dépôt `deltea/p3r-pause-menu` (SvelteKit) : « on laisse le choix à l'utilisateur de laisser
Revel télécharger les dépendances, les installer et construire le site ». Revient sur la partie « sans être
construit par Revel » de D-80.
- **Construire avec Revel** (bouton du panneau « Site à construire » de l'import Git, jamais automatique) :
  plan montré d'abord (outil reconnu, outils à télécharger et leur taille, gestionnaire de paquets, ce qui
  manque). Node.js : version LTS la plus récente de `nodejs.org/dist/index.json`, archive vérifiée par
  `SHASUMS256.txt`, rangée dans les données de Revel (`tools/node-<version>`) ; Hugo : édition « extended » de
  la dernière version, vérifiée par son fichier de sommes ; Jekyll : Ruby et Bundler du système seulement
  (absents : signalé, pas d'installation de Ruby par Revel). npm livré avec Node.js ; pnpm et Yarn par
  `corepack` à la version demandée par le dépôt ; `bun.lock` seul : npm, signalé (versions des dépendances
  choisies par npm). Construction dans une copie de travail (`tools/work`), variables d'environnement réduites
  (aucun jeton transmis), processus arrêtés avec tous leurs enfants à l'annulation, journal gardé et copiable en
  cas d'échec. Dossier produit trouvé (`build`, `dist`, `out`, `public`, `.output/public`, `_site`…) : importé
  comme un dossier ; site qui n'en produit pas (SvelteKit avec serveur, Nuxt, Remix…) : servi par sa propre
  commande d'aperçu sur `127.0.0.1`, lu comme un lien (7.4) puis ses fichiers client ajoutés. Outils gardés,
  retirables, paquets en cache vidables : Réglages › Outils de construction. Outils reconnus : SvelteKit,
  Next.js, Nuxt, Astro, Gatsby, Remix, React Router, Angular, SolidStart, Docusaurus, VitePress, Create React
  App, Vite, Parcel, webpack, Hugo, Jekyll. Sources :
  [index des versions de Node.js](https://nodejs.org/dist/index.json),
  [versions de Hugo](https://github.com/gohugoio/hugo/releases), [Corepack](https://nodejs.org/api/corepack.html).
- **Importer le site publié** : adresse trouvée dans le dépôt (`homepage` de `package.json`, fichier `CNAME`,
  `site` d'Astro, `baseURL` de Hugo, `url` de Docusaurus ou de Jekyll, liens du README, page GitHub Pages du
  dépôt) puis vérifiée en ligne (deux au plus) ; import comme un lien (7.4). Plus d'impasse : le panneau propose
  toujours la construction, le site publié s'il y en a un et le dossier produit s'il existe.
- **Lien d'un site animé par son programme** (SvelteKit, Next.js, Nuxt, Astro…) : modules JavaScript et leurs
  fichiers (imports, images et polices citées dans le code) gardés au lieu d'être retirés ; signalé dans le bilan.
- Vérifié le 17/10/2026 sur `deltea/p3r-pause-menu` : Node.js 24.22.0 téléchargé et vérifié, `npm install`,
  `vite build`, aperçu servi puis lu ; site importé identique à `p3r.deltea.space` (0,0 % de pixels différents,
  écran d'accueil et menu ouvert), seul écart : script de mesure d'audience de Vercel absent (sans effet visible).

**D-89 — 18/10/2026 — prise.** **Travail à plusieurs : dossier partagé du réseau local ou dépôt en ligne, au
choix, différences montrées à l'activation (Paramètres › Versions).** Demande de l'utilisateur : « On laisse le
choix, soit travailler sur le NAS/réseau local soit via Git ». Chacun travaille sur sa copie du site (son
ordinateur, hors ligne si besoin) puis synchronise : versions des autres récupérées, les siennes envoyées.
- **Seul** (par défaut) : versions dans le dossier du site. **Dossier partagé** : Revel crée
  `<dossier>/<nom-du-site>.git` (dépôt Git sans copie de travail, branche du site) puis y envoie les versions ;
  aucun compte, aucun prestataire ; chemins `\\serveur\partage\…`, lecteur réseau (`Z:\…`) ou dossier monté
  (`/Volumes/…`), lus par libgit2 (chemins UNC pris en charge :
  [`path_w32.c`](https://github.com/libgit2/libgit2/blob/main/src/util/win32/path_w32.c)). **Dépôt en ligne** :
  GitHub, GitLab ou tout dépôt HTTPS, comme avant (Pages CMS possible). Choisir l'un des deux ouvre une fenêtre
  qui compare les deux façons (versions communes, comptes, hors du bureau, contenus modifiés dans un navigateur,
  copies du site, prestataire), la façon choisie mise en avant, puis le dossier ou l'adresse ; « Seul » retire
  le dépôt commun après confirmation (versions gardées). Adresse à donner à l'équipe copiable ; les autres
  ouvrent le site avec Accueil › Importer › Git (bouton « Dossier… » pour un dossier partagé).
- **Fusion par endroit** du document du site (`site.revel.json`) et des autres fichiers JSON : modifications
  faites à des endroits différents (pages, gabarits, composants, éléments par leur identifiant, textes entre
  eux, règles de style par plages, réglages) combinées sans rien demander, y compris sur des lignes voisines ;
  ajouts des deux côtés au même endroit gardés tous les deux. Endroit modifié des deux côtés : liste « Page /,
  `<h1>.titre` — Ma version / Version reçue », choix un par un ou « Toutes les miennes / Toutes les reçues »,
  puis « Appliquer » (résultat enregistré et envoyé). Autres fichiers modifiés des deux côtés : un choix pour
  tous. Rien n'est changé tant que les choix ne sont pas appliqués.
- **Nom de l'auteur** : Réglages › Comptes › « Votre nom », inscrit sur les versions faites depuis cet
  ordinateur (qui a modifié quoi, y compris avec le dossier partagé, sans compte) ; sans nom : celui réglé dans
  Git, sinon le compte GitHub. Dépôt en ligne : un compte GitHub par personne, invité au dépôt ; les conditions
  de GitHub interdisent de partager un identifiant (« a single login may not be shared by multiple people »,
  [conditions d'utilisation, B.3](https://docs.github.com/en/site-policy/github-terms/github-terms-of-service)) et
  un dépôt privé gratuit accepte des collaborateurs sans limite de nombre
  ([documentation](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/repository-access-and-collaboration/inviting-collaborators-to-a-personal-repository)).
- Pas de serveur ni de service de Revel : le dossier partagé ne demande qu'un accès en lecture et écriture ; le
  NAS n'a pas besoin d'avoir Git. Limite : deux personnes ne voient pas les modifications de l'autre en direct
  (synchroniser pour les recevoir).

## Décisions ouvertes

**D-07 — à confirmer avant la première version publique.** Identifiant d'application `app.revel-studio.desktop`
(`tauri.conf.json`). Il détermine les dossiers de données et les mises à jour : le changer après diffusion casse la continuité.
Basé sur le domaine `revel-studio.app`, libre mais non enregistré.

**D-08 — ouverte.** Licence du code : propriétaire, MIT/Apache-2.0, GPL… Le dépôt est public depuis le 14/10/2026
(D-17) sans licence : tous droits réservés jusqu'à la décision. Une licence ouverte ne peut pas être retirée des
versions déjà publiées sous cette licence.

**D-09 — ouverte (étape 6).** Signature de code : certificat Windows (sinon avertissement SmartScreen), compte Apple Developer (sinon blocage Gatekeeper).
- Préparé le 17/10/2026 (6.7) : la construction des installateurs signe dès que les secrets et variables du
  dépôt sont déclarés (`.github/workflows/release.yml`, étape « Signatures et mises à jour ») ; sans eux,
  installateurs non signés sous Windows et signés « ad hoc » sous macOS.
- Options vérifiées le 17/10/2026, à choisir par l'utilisateur :
  - Windows, Azure Artifact Signing (ex-Trusted Signing) : 9,99 USD par mois pour 5 000 signatures ;
    ouvert aux organisations des États-Unis, du Canada, de l'Union européenne et du Royaume-Uni, aux
    personnes seulement aux États-Unis et au Canada ; certificats de 24 h renouvelés seuls
    (https://devclass.com/2026/01/14/code-signing-windows-apps-may-be-easier-and-more-secure-with-new-azure-artifact-service/).
  - Windows, certificat OV ou EV d'une autorité : la réputation SmartScreen se construit de la même façon
    avec l'un ou l'autre depuis 2024 (https://v2.tauri.app/distribute/sign/windows/).
  - macOS, Apple Developer Program : 99 USD par an ; seul moyen d'obtenir un certificat Developer ID et la
    notarisation ; sans lui, signature ad hoc et autorisation manuelle dans Confidentialité et sécurité
    (https://v2.tauri.app/distribute/sign/macos/).
- Options gratuites vérifiées le 17/10/2026 : Microsoft Store avec un paquet MSIX (inscription gratuite pour
  les particuliers, paquet re-signé par Microsoft, sans avertissement ; Tauri ne produit pas de MSIX, étape à
  ajouter ; https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) ;
  SignPath Foundation (projets sous licence open source approuvée par l'OSI seulement, éditeur affiché
  « SignPath Foundation » ; https://signpath.org/terms). macOS : aucune voie gratuite.
- **17/10/2026 — choix de l'utilisateur : pas de signature pour l'instant** (installateurs non signés sous
  Windows, signés ad hoc sous macOS). Les brouillons de versions antérieurs sont gardés comme historique du
  développement.

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

**D-46 — 16/10/2026 — prise (décision de l'utilisateur), remplacée par D-79 pour les langues livrées.** **Installateur.** Le modèle de traduction
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
