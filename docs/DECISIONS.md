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

**D-90 — 18/10/2026 — prise.** **Tout site s'édite depuis l'interface, quelle que soit son origine.** Demande de
l'utilisateur : « l'import d'un site externe doit avoir le même niveau de contrôle qu'un site natif à Revel » et
« quand tu te heurtes à une limite, tu crées l'outil ». Trois pièces : le **relevé** (`revel audit`, outil MCP
`check_site`, `revel_edit::editable`) liste ce que les panneaux ne savent ni créer ni régler (variables,
expressions calculées, conditions et listes écrites, attributs calculés) ; la **reconstruction** (`revel
rebuild`, `revel_render::flatten`) réécrit un site dans la forme des panneaux à rendu identique (données
connues d'avance écrites dans les calques, propriétés de composant déduites champ par champ, listes de
collection, flux RSS, réglages de page, liens « page actuelle », couleurs par valeur en classes) ; chaque
**import** (Eleventy, HTML, lien, Git construit, WordPress, Figma) passe par la reconstruction, le bilan dit ce
qui reste écrit dans le code (`revel import … --no-rebuild` garde l'import brut). Les assistants (MCP) reçoivent
la liste des formes de l'interface et doivent finir avec un relevé vide.

**D-91 — 18/10/2026 — prise.** **Formes de l'interface ajoutées pour les limites rencontrées.** Conditions par
clauses (rempli, vide, égal à, différent de, contient, plus / moins de N valeurs), toutes vraies ou une au moins ;
propriété de composant « Liste » ; liste de collection « Mettre en premier » (champ ou contenus choisis,
complétés, ou seulement s'ils remplissent la liste) ; classe tirée de la valeur d'un champ ; adresse de lien
écrite autour d'un champ ; nombre de valeurs d'un champ ; mots entre *astérisques* mis en valeur ; filtres de
liste (boutons, éléments filtrés, paramètre d'adresse) ; consentement aux cookies avec Google Analytics 4 après
accord ; onglets au survol avec vidéos du panneau affiché ; lecteur vidéo (YouTube, Vimeo, fichier) ;
interactions « un enfant au hasard » et « tourner » ; Échap referme un menu ouvert par une interaction. Chaque
forme est lisible dans le code (attributs `data-revel-*`, filtres `pick_first`, `emphasis`) et réglable dans
Styles.

**D-92 — 18/10/2026 — prise.** **L'en-tête des pages est écrit par Revel d'après les réglages.** Paramètres ›
Référencement : nom affiché du site (« Page | Nom »), description et image de partage par défaut, couleur du
navigateur, icônes, fiche de l'organisation (données structurées schema.org) ; réglages d'une page et du modèle
d'une collection : classe du corps, thème, type au partage. Revel écrit titre, partage (Open Graph, Twitter),
langue, date d'article, `max-image-preview:large`, icônes, flux RSS et données structurées quand le gabarit ne
les a pas. Les liens vers la page affichée reçoivent `aria-current="page"` (comme la classe `w--current` de
Webflow, [forum Webflow](https://discourse.webflow.com/t/force-apply-current-class-to-nav-bar-link/36871)) ; le
réglage « Actif sur les sous-pages » d'un lien couvre les pages dont l'adresse commence par la sienne.
Une balise déjà écrite par le gabarit ou par le code ajouté (site, page) n'est jamais écrite une seconde fois ;
le type annoncé par la page décide de la date d'article. À la reconstruction (`revel rebuild`, et donc à
chaque import), l'en-tête écrit à la main devient ces réglages, groupe par groupe (adresse, robots, langue,
icônes et couleur, flux, description, image, type, date, titre ; image, type et date de chaque page depuis son
code ajouté) : un groupe n'est remplacé que si le site produit reste le même, à l'ordre des balises près et
hormis les balises de partage que Revel ajoute là où la page n'en avait pas ; sinon il reste dans le gabarit.
Date de publication : réglage de la page (Apparence).

**D-93 — 18/10/2026 — prise.** **Alunites recréé dans la forme de l'interface, relevé vide.** Le script propre au
site est retiré : chaque comportement est un réglage (en-tête au défilement, menu, onglets « À la une »,
bandeaux défilants, filtres, lecteur vidéo, galeries, boussole, formulaire, consentement). Données devenues
collections (rubriques, clients) ; noms de l'équipe écrits comme ils s'affichent ; chiffres et pictogrammes en
propriétés du composant « Chiffres ». Écarts voulus avec la version Eleventy : le titre d'une page sans titre de
référencement devient « Titre du contenu | Nom du site » au lieu du seul nom du site, et les bandeaux défilent
aussi sur téléphone.

**D-94 — 18/10/2026 — prise.** **Outils pour les sites animés et sonores (exemple : menu de jeu p3r).**
Styles : ombre du texte (calques X, Y, flou, couleur, comme les ombres de bloc), point d'appui des
transformations (grille 3 × 3 ou valeur), découpe (`clip-path` : formes proposées, puis points du polygone
un par un) ; états « Actif » (`.is-active`, posé par les onglets, filtres, interrupteurs), « Dans un élément
actif » (`.is-active .x`), « Éteint » et « Dans un interrupteur éteint » (`[aria-pressed="false"]`).
Interactions : actions « Jouer un son » et « Lancer une musique » (fichiers des médias, un pris au hasard,
volume ; musique en boucle, une à la fois), interrupteurs « Musique » et « Sons » (élément du panneau
Ajouter, texte « On / Off » de leur état, choix gardé par le navigateur) ; onglets : touches du clavier
dans toute la page (flèches, W/A/S/D), son à chaque changement, plusieurs séries de panneaux qui suivent
les onglets dans l'ordre (texte, numéro) ; animation « Devenu actif » (rebond, saut, secousse, éclair,
tour) jouée quand l'onglet, le filtre ou l'interrupteur qui contient l'élément devient actif. Rien de
propre à un site : chaque réglage est un attribut `data-revel-*` lu par le script commun. Vérifié en
recréant p3r.deltea.space avec les seuls outils des assistants (MCP) : relevé vide, 0,01 % d'écart sur
l'écran d'accueil, 0,2 à 0,34 % sur le menu (1440 × 900) ; icône « flèche épaisse » ajoutée, élément
« Icône » réglable par son nom pour les assistants.

**D-95 — 18/10/2026 — prise.** **Redirections écrites dans le site produit.** La liste de Paramètres ›
Navigation › Redirections (et celles qu'ajoute un changement d'adresse de page) n'était qu'enregistrée : la
génération l'écrit désormais. Apache : règles `mod_rewrite` dans `.htaccess` (créé s'il n'existe pas, sinon
placées avant la première règle du fichier), seules capables de lire les paramètres d'une ancienne adresse
(`/?p=12`) — `Redirect` de `mod_alias` ne lit pas la chaîne de requête
([S. Carletti](https://simonecarletti.com/blog/2009/01/apache-query-string-redirects/)). Tout hébergeur
(GitHub Pages ignore `.htaccess`) : page de redirection immédiate à l'ancienne adresse (`meta refresh` à
0 seconde, lu comme permanent par Google,
[Google Search Central](https://developers.google.com/search/docs/crawling-indexing/301-redirects)), avec
`noindex` et adresse canonique ; jamais à la place d'une page existante. Les assistants (MCP) lisent la liste
dans `get_site` et la remplacent par `apply_commands` (`set_redirects`).

**D-96 — 18/10/2026 — prise.** **Propriétés du gabarit.** Une valeur qui change d'une page à l'autre dans un
gabarit (masquer l'appel à contacter, un sous-titre, une image d'en-tête) est une propriété du gabarit, comme
celle d'un composant l'est pour chaque utilisation : déclarée par le gabarit (texte, image, lien,
interrupteur ; valeur par défaut), réglée dans les paramètres de chaque page (« Propriétés du gabarit »,
avec ajout, nom et retrait), reliée aux éléments du gabarit depuis l'onglet Styles (texte, image, lien,
visibilité, comme dans un composant ouvert). Une utilisation de composant dans le gabarit peut reprendre une
propriété du gabarit, inversée au besoin (`not x`). Une propriété ajoutée depuis une page va au gabarit le
plus extérieur de sa chaîne, lisible par tous ceux qu'il contient. À la reconstruction, une variable de page
lue par son gabarit (`{% if not hideFooterCta %}`, champ de front matter d'Eleventy, de Hugo ou de Jekyll)
devient une telle propriété (interrupteur pour un booléen, image ou lien selon l'endroit où elle sert),
avec un libellé lisible (« Hide footer cta ») modifiable ; les pages gardent leur valeur. Une liste ou un
objet reste une variable de page. Assistants (MCP) : `set_layout_fields`, `set_page_prop`.

**D-97 — 18/10/2026 — prise.** **Reconstruction étendue (imports d'Eleventy et autres générateurs).** Formes
ajoutées à la reconstruction, chacune gardée seulement si le site produit reste identique (fichiers
comparés gabarit par gabarit, page par page, composant par composant) :
- réglage de page tiré des données (`seo = pages["agence"].seo`) écrit dans la page ; absent des données :
  retiré ;
- plan du site écrit à la main (boucle sur une collection) remplacé par celui de Revel quand il liste les
  mêmes adresses avec les mêmes dates ; réglage ajouté : « Dates de mise à jour » (`lastmod`) dans
  Paramètres › Référencement ;
- « suggestions du contenu s'il y en a assez, sinon les plus récents » et « contenus choisis, sinon les plus
  récents », écrits avec des variables : liste de collection « Mettre en premier » (seulement s'ils
  remplissent la liste, ou complétés) ;
- contenus choisis un à un dans plusieurs collections (`(projets | by_slug(s)) or (actualites |
  by_slug(s))`) : liste de collection à plusieurs collections (nouvelle forme du panneau : « + Autre
  collection », champs des collections réunis) ; premier contenu lu à part (`liste[0].data.cover`) :
  liste d'un seul contenu ;
- contenu relié lu directement (`(collections.equipe | by_slug(contact_person)).data.job`) : liste des
  contenus reliés par le champ ;
- valeur tirée d'un tableau gardée dans une variable du même nom et lue par un style : classes par valeur
  (D-91) ;
- `(a or b)` entre doubles parenthèses simplifié ; propriété de composant reliée à un champ avec un champ de
  repli et une écriture de date : affichée et réglable dans l'utilisation (« Date, sinon Date affichée »,
  écriture choisie).
Site cobaye d'Alunites (Eleventy) : 331 → 63 points dans le code, styles et captures identiques sur 13
pages à 1280 et 375 px, fichiers du serveur et consignes aux moteurs identiques en recette. Restent surtout
le `.htaccess` écrit en boucles sur les contenus, des valeurs calculées propres au site (noms mis en forme,
échappements faits à la main, HTML assemblé), le lecteur vidéo propre au site et les attributs lus par son
script ; une liste « suggestions » reste en variables parce qu'une actualité se suggère elle-même (le rendu
changerait).

**D-98 — 19/10/2026 — prise.** **Environnements de publication gardés à la reconstruction.** Défaut corrigé de
la 0.6.2 : la reconstruction écrivait les valeurs de la production à la place des variables d'environnement
(`{% if env.staging %}`), si bien qu'une recette publiée perdait son `noindex` et son en-tête
`X-Robots-Tag`. Quand le site a plusieurs environnements, `env` reste lu à la publication ; chaque réécriture
n'est gardée que si les fichiers produits restent identiques dans tous les environnements ; la CI compare
en recette les fichiers du serveur et les consignes aux moteurs (`tools/compare-robots.py`). Dans
l'interface, une condition peut lire une variable d'environnement (« Environnement › staging ») et les
propriétés du gabarit ; le texte d'un fichier du serveur écrit en partie par des boucles ou des conditions
est montré sans être modifiable (l'enregistrer comme texte seul effaçait ces parties) ; dans le relevé, le
texte d'un tel fichier n'est plus compté comme du HTML écrit.

**D-99 — 19/10/2026 — prise.** **Média au survol : outil distinct de la liste qui le pilote.** Demande de
l'utilisateur (« two distinct tools ») : la vidéo de fond du héros d'Alunites était appariée aux éléments de
la liste par leur position, sans indication. Désormais la liste est une liste de collection ordinaire
(contenus choisis dans plusieurs collections, champs du CMS) et le média au survol un outil à part :
élément média (`data-revel-hover-media="nom"`, Ajouter › Média au survol) et éléments qui le changent
(`data-revel-media-for`, adresse du média et image d'attente lues dans un champ du contenu, réglées dans
le panneau : « Affiché dans », source, champs, bouton pour aller de l'un à l'autre). Le premier élément est
montré au chargement (image d'attente écrite à la génération), l'élément montré reçoit `is-active`, le
média démarré `is-ready` ; l'action « Jouer » met en pause (`is-paused`) ; mouvement réduit : rien ne démarre
seul. États ajoutés dans Styles : « Prêt », « Dans un élément en pause ».

**D-100 — 19/10/2026 — prise.** **Informations du site partagées par toutes les pages.** Paramètres ›
Informations du site : e-mail, téléphone, adresse, réseaux, toute valeur ajoutée (libellé modifiable),
et menus (liste de liens, `site.menus.<nom>`). Un texte, un lien (`mailto:`, `tel:` avec le numéro écrit
pour l'appel, plan avec l'adresse), une image, une liste de liens ou une condition se relient à une
information depuis l'onglet Styles ; une liste de liens écrits devient un menu du site. Valeurs gardées
dans le document (ou dans `_data/site.yml` quand le site en a un). La reconstruction garde les lectures
`site.*` au lieu d'écrire leur valeur, convertit les numéros d'appel et les adresses de plan écrits à la
main, et l'année en cours.

**D-101 — 19/10/2026 — prise.** **Ergonomie de l'onglet Styles : rôles, onglets, conteneurs.** Un élément
sans rôle dans un élément interactif reçoit un seul choix « Rôle », groupé par élément (onglets,
carrousel, lecteur vidéo, formulaire, filtres de liste, consentement, interrupteur des sons) au lieu d'une
liste par élément ; une fois choisi, seuls les réglages de ce rôle restent. Onglets : chaque onglet dit le
panneau qu'il montre et inversement (sélectionnables, nombres qui ne correspondent pas signalés). Les
attributs réglés par un contrôle ne sont plus répétés dans « Autres attributs ». Calques : un élément en
ligne qui contient des éléments s'affiche comme un conteneur (plus de « T ») ; la recréation d'Alunites
utilise des conteneurs (`div`) pour l'image et le texte des cartes.

**D-102 — 19/10/2026 — prise.** **7.10 terminée : un site Eleventy importé n'a plus rien dans le code.**
Outils créés pour les dernières limites :
- fichier du serveur (`.htaccess`, `robots.txt`) modifié comme texte avec ses balises (valeurs, conditions
  sur l'environnement, listes de contenus) dans les paramètres de la page, relu à l'enregistrement ; un
  texte illisible est refusé avec sa raison ;
- valeur reliée d'un attribut (Inspecteur › Attributs) : valeur choisie dans une liste (champ, propriété,
  information du site, nombre de contenus), texte avant et après, « seulement si la valeur est remplie »,
  texte si vide ; propriété de style reliée (`--dot` ← Couleur) ;
- texte relié : écriture propre au site (« Prénom NOM »), valeurs d'un champ liste séparées, texte si
  vide ; conditions : premier ou dernier de la liste, contenu qui a une page ;
- liste de collection : un contenu mis en premier reste montré même s'il est le contenu affiché
  (`pick_first(…, page.url)`).
Reconstruction, chaque réécriture gardée seulement si les fichiers produits restent identiques (sauf les
couleurs choisies dans un tableau, en classes comme avant) : choix écrit d'avance replié
(`(nom if (1) == 1 else "")`) ; variables d'un composant calculées d'après une valeur de l'utilisateur →
propriétés (déduction répétée), `x or 'Vidéo'` garde son texte de repli ; image dont un réglage dépend d'une
condition → une image par cas ; liste de données lue par adresse (`rubriques.items | find_by('slug', s)`) →
collection du CMS (contenus écrits, `.pages.yml` complété, liste de choix aux mêmes adresses changée en
référence) ; boucle sur un champ autour de contenus reliés → liste des contenus reliés ; composant qui
reçoit une liste construite pas à pas ou du HTML assemblé → déplié à sa place (retiré s'il ne sert plus) ;
liste construite pas à pas → parcourue directement ; HTML assemblé dans un texte → éléments ; condition
connue d'avance → son contenu. Site cobaye d'Alunites : 273 → 0 point ; 13 pages × 1280/375 px × 2
préférences de mouvement : aucun écart de styles calculés ni de captures ; `.htaccess` et consignes aux
moteurs identiques en recette.

**D-103 — 19/10/2026 — prise.** **Plan du site nodal.** Nouvel onglet « Plan du site », avant les
Paramètres : toutes les pages et ce qui les relie, sur un plan infini à points (comme un canevas nodal de
type Weavy : fond glissé ou molette pour se déplacer, Ctrl+molette ou pincement pour zoomer, F pour tout
voir, minicarte cliquable). Le relevé des liens est dans `revel-edit` (`linkmap`), utilisé aussi par l'outil
`get_site_map` des assistants :
- nœuds : pages ; collections qui ont une page par contenu (contenus dépliables) ; menus du site,
  composants et gabarits qui portent des liens (« parties communes », avec le nombre de pages qui les
  affichent) ; adresses du site sans page (en rouge) ; autres sites (par domaine, masqués par défaut) ;
- liens : lien écrit (`href`, `{{ '/x/' | url }}`, relatif, adresse absolue du site, adresse d'une autre
  langue), valeur d'un composant (`href: "'/contact/'"`), lien d'un menu du site, liste de contenus (vers la
  collection), liste d'un menu (vers le menu), redirection ; une adresse redirigée mène à sa destination ;
  plusieurs liens entre deux nœuds : un trait plus épais ; un gabarit propre à une collection est le
  modèle de ses pages (ses liens partent de la collection) ;
- rangement automatique d'après les adresses (page parente à gauche ; une collection sous la page qui la
  liste) ; un nœud déplacé garde sa place dans le site (`library.map`, commande `set_map_positions`,
  annulable), « Ranger » rend tout au rangement automatique ;
- ajuster les liens : bout du trait sélectionné tiré sur une autre page, ou « Mener à » dans le panneau
  (lien écrit, lien de menu, redirection ; requête et ancre gardées) ; point d'un menu tiré sur une page :
  lien ajouté au menu ; point d'une page : lien ajouté à la fin de la page (« Montrer dans l'éditeur ») ;
- panneau : nom, adresse (redirection de l'ancienne si la page est publiée), brouillon, ouvrir dans
  l'éditeur (double-clic), nouvelle page dedans, dupliquer, supprimer ; adresse sans page : créer la page ou
  rediriger l'adresse ; liens entrants et sortants (« Montrer dans l'éditeur » sélectionne l'élément) ;
  pages sans lien qui y mène signalées.
Ctrl+Z annule dans le plan (même historique que l'éditeur).

**D-104 — 19/10/2026 — prise.** **Paramètres réorganisés.** Paramètres du site et réglages de Revel ont la
même forme : un rail à gauche ; la rubrique ouverte prend la teinte de la page et s'y raccorde comme
l'onglet actif de la barre supérieure (contour qui en fait le tour, coins incurvés) et liste ses parties
(partie à l'écran marquée au défilement, clic : la partie montrée) ; chaque partie a son titre et sa
présentation à gauche (restent visibles en défilant), ses réglages à droite, au centre de la page (plus de
colonnes de hauteurs différentes côte à côte ni de grande zone vide). Partie vide retirée (listes de liens
des gabarits absentes) ; flux RSS proposés seulement pour les collections qui ont des pages ; informations du
site en colonnes alignées (nom, valeur, utilisations, retrait), retrait d'une valeur utilisée expliqué.

**D-105 — 19/10/2026 — prise.** **Calque masqué caché dans le canevas, champs de média.** La règle qui cache
les calques masqués est posée par le script du canevas dès son chargement (un calque masqué sur place, sans
nouveau rendu, disparaît aussitôt) et ne compte plus dans l'en-tête comparé (plus de rechargement). Média au
survol : « Vidéo de chaque contenu » ne propose que les champs qui peuvent en donner une (fichier, ou texte
dont les valeurs sont des adresses de vidéo `.mp4`, `.webm`, `.mov`… ; sans valeur encore : nom qui le dit),
l'image d'attente que des champs d'image ; aucun champ : message qui dit quoi ajouter au CMS. Valeurs
d'attribut qui contiennent `>` dans les vues de l'interface toujours entre accolades (sinon la balise se
fermait et le code s'affichait). Texte d'un fichier du serveur ou d'un flux qui commence par une lettre
accentuée après une balise : coupé entre deux caractères (le rendu s'arrêtait).

**D-106 — 21/10/2026 — prise.** **Profil, retour au site, polices dans Général, listes des médias.**
- Profil en haut à droite, à côté de « Publier » : la photo seule (le nom en bulle d'aide) ; un clic ouvre
  la fenêtre du profil : changer la photo (image de l'ordinateur recadrée en carré de 160 px, gardée dans
  les réglages de Revel, `avatar`), reprendre la photo GitHub ou la retirer, nom (celui qui signe les
  versions, D-89), compte GitHub, lien vers les réglages. Sans photo ni nom choisis : ceux du compte GitHub
  connecté (`github.com/<compte>.png`) ; sans compte : initiales, sinon une silhouette. Mêmes réglages dans
  Réglages de Revel › Profil et comptes.
- Réglages de Revel ouverts depuis un site : « Revenir à <site> » est un bouton de la barre supérieure, à
  côté du nom du site (un clic), et n'est plus dans le menu du site.
- Polices : partie de Paramètres › Général (rubrique « Polices » retirée du rail).
- Tri et filtre des médias : listes ouvertes comme les autres listes de Revel (fond relevé, coins arrondis,
  coche) ; une règle `appearance: none` laissait la liste du système, claire et illisible sur le thème
  sombre. Sélecteur personnalisable de Chromium 135 et suivants (WebView2).

**D-107 — 21/10/2026 — prise.** **Plan du site : médias, réseaux et liens externes vérifiés ; corrections.**
Demande de l'utilisateur : montrer, en plus des parties communes et des autres sites, les médias du CMS et les
liens externes (vidéos Vimeo, YouTube, réseaux sociaux) « afin de monitorer le fait que les liens soient
toujours accessibles et pour facilement les mettre à jour ».
- Groupe « Médias et réseaux » : un nœud par plateforme (`media:<plateforme>`, `revel_edit::linkmap::PLATFORMS` :
  vidéos YouTube, Vimeo, Dailymotion, Twitch ; sons SoundCloud, Spotify ; réseaux Instagram, Facebook,
  LinkedIn, X, TikTok, Pinterest, Behance, Threads, Bluesky, WhatsApp). Les autres domaines restent dans
  « Autres sites ». Adresses relevées : liens, médias intégrés (`src` des `<iframe>`, `<video>`, `<audio>`,
  `<source>`, `<embed>`), valeurs de composant, menus, et désormais les champs et le texte des contenus du
  CMS (pointeur JSON du champ) et les informations du site ; une adresse calculée (`…/video/{{ vid }}`) n'est
  pas une adresse écrite.
- Vérification par Revel (`revel_core::links`, sans service extérieur) : vidéo YouTube ou Vimeo par la réponse
  oEmbed de la plateforme (une page de vidéo supprimée répond quand même : vérifié, YouTube 200 / 400,
  Vimeo 200 / 404) ; autre adresse : premier octet demandé (`Range: bytes=0-0`), redirections suivies.
  États : en ligne, déplacée (adresse finale proposée), introuvable, privée, non vérifiable (page de
  connexion, 403, 429, 999 : LinkedIn et Instagram refusent les vérifications automatiques, l'adresse n'est
  pas morte pour autant), en erreur, sans réponse. Résultats gardés dans `.revel/links.json` (hors des
  versions), refaits au-delà d'un jour ou sur « Vérifier ». Pastille de l'état le plus grave sur chaque
  nœud, « N adresses à revoir » dans la barre (montre les deux groupes).
- Panneau d'une plateforme ou d'un autre site : adresses, à revoir d'abord, chacune avec son état, les
  endroits où elle sert (page, contenu et champ, information du site), ouvrir, « Remplacer » partout où elle
  sert (`linkmap::replace_url`, une seule étape d'annulation), « Utiliser cette adresse » pour une page
  déplacée. Dans un contenu du CMS, l'adresse est remplacée dans le fichier même (commande
  `ReplaceInEntry`, mise en forme gardée, adresse entière seulement), inverse au texte exact.
- Outils MCP `check_links` et `replace_link` (un assistant surveille et corrige les liens).
- Corrections : fond de la rubrique active des paramètres (accent étendu) étendu à tout l'onglet quand elle
  n'a pas de parties ; bouton « Ranger » centré dans la barre du plan (`.btn--small` se calait en haut) ;
  zoom du plan avec Alt+molette, comme le canevas d'édition.
Sources : [HTTP 999 (LinkedIn)](https://http.dev/999), [oEmbed des vidéos privées — Vimeo](https://help.vimeo.com/hc/en-us/articles/12427906892689-Use-oEmbed-with-private-videos).

**D-108 — 21/10/2026 — prise.** **Synchronisation automatique du travail à plusieurs.** Demande de
l'utilisateur, après comparaison des façons de faire (édition sur GitHub sans copie, copie partielle, copie
complète actuelle) : garder la copie complète, actualiser quand le dépôt commun change sans enregistrer de
version, garder Ctrl+Z pour la session (ses propres modifications seulement), s'arrêter et le dire en cas
de conflit.
- Vérification toutes les minutes (`revel_git::versions::incoming` : liste des branches du dépôt commun,
  rien n'est téléchargé ; 444 octets mesurés). Échange réduit aux objets manquants (mesuré sur Alunites :
  texte changé dans le document de 600 Ko, 546 octets ; image ajoutée de 1,3 Mo, 1,3 Mo), seuls les
  fichiers changés réécrits.
- Réception (`receive`) à la première pause d'édition (3 s sans saisie, aucun texte en cours d'édition dans
  le canevas ni champ actif), sans enregistrer de version : fusion à trois du dossier (changements en cours)
  avec la version reçue, par endroit pour le document ; la version actuelle devient la dernière reçue et les
  changements en cours restent à enregistrer. Versions faites des deux côtés : fusion enregistrée si rien
  n'est en cours, sinon au prochain enregistrement. Les versions enregistrées partent aussitôt
  (« Enregistrer une version » synchronise ; une version restée hors ligne part à la vérification suivante).
- Historique d'annulation gardé à la réception (`History::received`) : avant d'annuler ou de rétablir une
  étape, elle est appliquée puis défaite sur une copie ; si le document ne revient pas à l'identique (même
  endroit changé par un collègue, position décalée), l'étape est abandonnée avec un message et la suivante
  reste. Une étape qui écrit un fichier reçu (contenu, données, schéma) est abandonnée. Même principe que
  Figma et Google Slides (on n'annule que ses propres modifications).
- Conflit (même endroit modifié des deux côtés) : rien n'est changé, la synchronisation se met en pause et
  le dit (« La synchronisation n'a pas pu aboutir : N endroits… ») ; « Vérifier » ouvre Paramètres ›
  Versions, qui enregistre et montre les endroits à choisir ; « Plus tard » laisse travailler, pause
  signalée.
- Indicateur en haut à droite (sites à dépôt commun) : état, dernière vérification, versions reçues de qui,
  « Synchroniser » ; interrupteur « Synchronisation automatique » dans Paramètres › Versions (réglage de
  l'ordinateur, `auto_sync`, actif par défaut). Pas de serveur : une notification de GitHub demanderait un
  service de Revel en ligne.
Sources : [git fetch-pack](https://git-scm.com/docs/git-fetch-pack.html), [Undo/redo en travail à
plusieurs — Liveblocks](https://development.liveblocks.io/blog/how-to-build-undo-redo-in-a-multiplayer-environment.md),
[Multiplayer editing in Figma](https://www.figma.com/blog/multiplayer-editing-in-figma/).

## Décisions ouvertes

**D-109 — 24/10/2026 — prise.** **Import Framer : modules du site et CMS.** Demande de l'utilisateur (site
matv.framer.website : animations, projets au-delà des six premiers, survol, barre de progression et menus
dépliants perdus à l'import). La page publiée par Framer n'est que le premier rendu de React ; le reste est
relu dans les modules copiés avec le site (`_externe/framerusercontent.com/**/*.mjs`, `framer.rs`) :
- apparition au défilement (`__framer__enter`, `__framer__animate`) → `data-revel-appear` ; états de survol
  (variantes `…-hover` et règles `.hover`) → règles `:hover` ;
- barre de progression de code (`ScrollProgress`) → barre Revel (D-110) ; Lenis → défilement doux (D-110) ;
- CMS : fichiers `.framercms` (adresse `/modules/` → `/cms/`, format binaire gros-boutiste décodé :
  nombre de contenus, champs nommés et typés — texte, texte riche, image, lien, date, liste, objet) et
  schéma du module de la collection → collection Revel (`content/projects/*.md`, ordre de Framer), gabarit
  tiré de la page la plus complète (champs reliés par valeur, galerie reconstruite en liste, parties
  facultatives sous condition quand leurs champs sont vides ensemble), listes des autres pages reliées
  à la collection (tous les contenus, plus la limite de pagination du rendu publié), maçonnerie reprise ;
- langues : table `{code,id,name,slug}`, pages `/fr/…` alignées sur les pages principales (textes et
  attributs ; paragraphe mis en forme autrement découpé aux tirets), contenus traduits
  `content/fr/projects` (D-35) ; un même texte traduit différemment dans une page et un gabarit garde la
  traduction de la page (une traduction par texte, avertissement) ;
- vidéos copiées avec le site : adresse de la copie (`/_externe/…`, D-36) ; vidéo lancée seule coupée
  (`muted`, posé par le script de Framer) ; texte riche du CMS stylé comme dans Framer (conteneur
  `rich-text-wrapper` avec les styles de texte de chaque balise, règles de base `p.framer-text` étendues
  à `.rich-text-wrapper > p`).
Mesuré sur matv : différence de pixels 0,36 % (1280 px) et 0,07 à 1,9 % (375 px, bandeau de logos et vidéos
en mouvement exclus), textes visibles identiques en anglais.

**D-110 — 24/10/2026 — prise.** **Défilement doux, barre de progression, maçonnerie.** Outils manquants pour
refaire matv dans l'interface :
- Défilement doux (Paramètres › Général, `settings.smooth_scroll`) : la molette fixe une cible, la page ou la
  colonne qui défile sous le pointeur glisse vers elle ; clavier, barre et toucher inchangés ; coupé pour
  « moins de mouvement ».
- Barre de progression (panneau Ajouter) : `data-revel-progress="parent|page"` (réglage « Suit » : colonne
  qui la contient ou page), variable `--revel-progress` de 0 à 1, remplissage
  `width: calc(var(--revel-progress, 0) * 100%)`.
- Maçonnerie (bouton de la grille) : `data-revel-masonry`, lignes de 1 px, chaque élément placé
  (colonne, première ligne) et couvrant `round(hauteur + écart)` lignes, dans la colonne la moins haute
  ou, « Colonnes à tour de rôle » (`="order"`, colonnes de Framer), dans la colonne suivante ; le
  dernier élément de chaque colonne sans l'écart qui le suit (même hauteur qu'une grille ordinaire) ;
  recalculé au changement de taille, à l'ajout d'un élément et au réglage (grilles suivies dans le
  canevas : retirer la maçonnerie rend la grille ordinaire).
- Défilement doux : pas d'au moins 1 px (les derniers pixels d'une colonne étaient arrondis et jamais
  atteints) ; apparition au défilement jouée aussi pour un élément resté dans le dernier dixième de
  l'écran quand sa page ou sa colonne ne peut plus défiler (bouton en bas de la colonne des projets).
- Interrupteurs de l'interface : la case passe au-dessus de la poignée (un appui sur la poignée d'un
  interrupteur hors d'un libellé ne basculait pas).
- Aperçu de l'application : morceaux de fichiers servis (`Range`, réponse 206) pour la lecture des vidéos,
  comme l'exemple de diffusion de wry.
Source : [wry, exemple streaming](https://docs.rs/crate/wry/0.56.0/source/examples/streaming.rs).

**D-111 — 24/10/2026 — prise.** **Section affichée et variantes au clic.**
- Navigation qui suit la section affichée (réglage de l'élément qui contient des liens `#section`,
  `data-revel-spy="section"`) : le lien de la section en cours (dernière dont le haut a passé 40 % de la
  fenêtre, colonnes à défilement propre comprises) reçoit `is-active`, l'élément `data-revel-section`.
  Import Framer : variantes choisies par `__framer__targets` → règles
  `.racine[data-revel-section="work"] .élément`.
- Cible d'interaction « élément de la classe qui le contient » (`closest:.classe`) : un appui ouvre sa carte
  seulement, dans une liste de cartes semblables.
- Import Framer des variantes au clic (cartes « Outils ») : le rendu du composant est lu dans son code
  (`framer_jsx.rs` : appels `jsx(balise, props)`, conditions de variante, surcharges `…({id:{…}})`, textes
  des propriétés de l'utilisation, images de fond, rapport `fitImageDimension`) ; parties de la variante
  ouverte ajoutées à la page et masquées hors de `.is-active`, règles `.framer-v-ouverte` réécrites en
  `.framer-v-défaut.is-active`, interaction `click class:is-active closest:.framer-v-défaut` sur les
  éléments appuyés, images copiées. Mesuré sur matv : les quatre cartes ouvertes à 1,3–2,7 % de différence
  de pixels avec le site en ligne (lissage du texte).

**D-112 — 25/10/2026 — prise.** **Import Framer dans la forme de l'interface : composants et noms.**
- Composants Framer utilisés plusieurs fois dans une page, même variante et même structure
  (`framer_components.rs`) : composant Revel nommé comme dans Framer (`displayName`), ce qui change
  d'une utilisation à l'autre (textes, images, liens, textes alternatifs) devient une propriété nommée
  d'après le calque ; chaque utilisation garde ses valeurs. Sur matv : Stats (4 utilisations),
  Experience (2), Primary (2).
- Classes d'empreinte de Framer (`framer-1p11shv`) renommées d'après les calques (`intro`,
  `about-title`, rôle « text » ou « image » sans nom de calque) dans les pages, gabarits, composants,
  la feuille du site et les cibles d'interaction (`framer_names.rs`) ; 465 classes sur matv, rendu
  inchangé.

**D-113 — 25/10/2026 — prise.** **Gestionnaire de classes.** Panneau « Classes » (à droite, avec Styles et
Interactions) : chaque classe du site avec le nombre d'éléments qui la portent, ses éléments (sélectionnés d'un clic,
page, gabarit ou composant ouvert) et ses règles ; recherche ; « Renommer » partout (règles et blocs CSS de la feuille,
attributs `class` des éléments, utilisations de composants et images, cibles d'interaction `.classe`,
`closest:.classe`) en une commande annulable (`Command::RenameClass`) ; « Retirer » une classe (éléments et règles,
une règle à plusieurs sélecteurs garde les autres) ; filtre et retrait groupé des classes inutilisées (règles sans
élément). Double-clic sur une pastille du panneau Styles : renommer. Classes d'état posées par les scripts (`is-…`,
`rv-…`) non listées. Assistants : `list_classes`, `rename_class`, `remove_class` (`crates/revel-edit/src/classes.rs`).
Les classes écrites dans les contenus du CMS (texte riche) et le code ajouté ne sont pas relevées.

**D-114 — 25/10/2026 — prise.** **Liste à affichage progressif.** Réglage « Afficher d'abord » d'une liste de
collection (`data-revel-more="6"` sur l'élément qui contient la liste) : le site publié montre les 6 premiers
éléments, puis, quand le bas de la liste arrive à l'écran (dans la page ou dans la colonne qui défile), un indicateur
de chargement tourne 0,45 s et les 6 suivants apparaissent (sans attente si le visiteur demande moins de mouvement).
Tous les contenus sont dans la page (référencement, lecteurs d'écran, sans script) ; le canevas montre tout ;
compatible avec la maçonnerie (éléments masqués ignorés). Comme la pagination de Framer sur matv.

**D-115 — 25/10/2026 — prise.** **Couleurs du site et mode sombre.** Paramètres › Général › Couleurs : variables de
couleur de `:root` (celles du sélecteur de couleur) listées par nom lisible, valeur claire réglable (pastille ou
texte), ajout, retrait (règles qui l'utilisent signalées). Interrupteur « Mode sombre » : règle
`@media (prefers-color-scheme: dark) { :root { … } }` créée avec les valeurs claires, une colonne « Sombre » par
couleur (vide : valeur claire) ; le site suit le thème de l'appareil du visiteur. Canevas : bouton « Aperçu en mode
sombre » de la barre d'outils quand le site a des couleurs selon le thème : les requêtes `prefers-color-scheme` des
feuilles du cadre deviennent vraies ou fausses (le cadre suivrait sinon le thème du système, différemment selon le
moteur de WebView2, WebKitGTK ou WKWebView) ; au départ, le thème de l'appareil.

**D-116 — 25/10/2026 — prise.** **État « Ouvert » d'un accordéon, GIF animés.** Panneau Styles : états « Ouvert
(accordéon) » (`[open]`) et « Dans un accordéon ouvert » (`[open] .x`) ; pendant le réglage, le canevas ouvre
l'accordéon de la sélection et le referme ensuite. Images : un GIF animé est servi tel quel (ses variantes AVIF,
WebP ou GIF redimensionné n'en gardaient que la première image) ; nombre d'images lu dans la structure du fichier.

**D-117 — 25/10/2026 — prise.** **Site matv recréé.** Recréation à la main de https://matv.framer.website dans la forme
de l'interface (demande du 25/10/2026) : collection `projects` (pages des projets, cartes de l'accueil) ; d'abord
des collections pour les clients, services, outils, parcours et classements, remplacées le jour même par des
composants (D-119) ; composants à propriétés reliées aux champs (Button, Stat, Service, Toolset, Tool,
Experience, Rank, Project card, Fact, Contact) ; classes nommées par rôle (`profile-name`, `toolset-head`,
`project-card-media`) ; couleurs du site claires et sombres ; défilement de logos (Défilement), accordéons des
outils, maçonnerie « colonnes à tour de rôle », 6 projets puis la suite ; informations du site (courriel,
téléphone, menu « social ») ; anglais et français. Écarts voulus : espacements des cartes « Outils » réguliers
(Framer les décale), lettre « M » du nom en image avec le nom complet pour les lecteurs d'écran.

**D-118 — 25/10/2026 — prise.** **Emplacement des composants.** Un élément d'un composant peut être son
emplacement (`data-revel-slot`, interrupteur « Emplacement du contenu » de Styles dans le composant ouvert ; un seul
par composant). Chaque utilisation peut y mettre son propre contenu (`children` du nœud `use`, nœuds de la page :
calques sous l'utilisation, sélection et texte modifiables dans le canevas, ajouts, glisser-déposer) ; sans contenu
propre, l'emplacement montre le contenu du composant. Styles d'une utilisation : « Modifier ici » copie le contenu du
composant dans l'utilisation (propriétés lues remplacées par les valeurs de l'utilisation), « Revenir au contenu du
composant » le retire. Détacher place le contenu propre dans la copie. Canevas : un emplacement rempli porte
`data-rv-slot-filled` ; ce qu'il contient se sélectionne à part, le reste de l'utilisation reste un bloc. L'attribut
n'est pas écrit dans le site publié. Comme les « slots » de Webflow et l'enfant d'un composant de Framer.

**D-119 — 25/10/2026 — prise (règle de l'utilisateur).** **Le CMS sert aux pages, les composants aux blocs.** Une
collection est faite pour des contenus qui ont chacun leur page (projets, articles) et pour les listes qui montrent
ces pages ; les blocs d'une page, même répétés (services, outils, classements, logos), sont des composants placés sur
la page, leur contenu dans leurs propriétés (valeurs par défaut dans le composant, remplacées par utilisation dans
Styles) et leur emplacement. Nouvelle collection : créée avec son modèle de pages et l'adresse
`/<collection>/{{ slug }}/` (une seule action, annulable d'un coup). Instructions des assistants (MCP) réécrites dans
ce sens. Site matv refait ainsi (D-117) : seule la collection `projects` reste ; services, boîtes à outils, outils,
parcours, classements et logos sont des composants et des éléments de la page.

**D-120 — 25/10/2026 — prise.** **Classes proposées, états selon l'élément.** Le champ « Ajouter une classe… »
propose au focus les classes du site les plus probables pour l'élément (`revel_edit::classes::suggest`) : celles des
voisins de même balise (carte d'une liste de cartes), celles nommées d'après la classe du parent ou de même racine
(`carte-titre` dans `carte`, `text-small` pour un `text-body`), celles des autres voisins, celles de la même balise
ailleurs, puis les autres, les plus utilisées d'abord ; groupes séparés par un trait, raison en info-bulle ; la
saisie filtre, « Créer « nom » » pour un nom nouveau ; ↑ ↓ Entrée Échap. Le choix d'état n'est plus une liste de
treize états : Normal et Survol toujours ; Appui pour ce qui réagit au clic ; Focus clavier pour ce qui se prend au
clavier ; Page actuelle pour un lien ; Actif, Parent actif, Ouvert, Accordéon ouvert, Éteint, Interrupteur éteint,
Média prêt, Vidéo en pause seulement quand la page les pose sur l'élément ou ce qui le contient (le canevas les
relève : onglets, interrupteurs, médias au survol, élément pris au hasard, `details`, vidéo arrêtée) ; un état qui a
déjà des styles pour la classe reste proposé, marqué d'un point bleu. Libellés courts, explication en info-bulle.

**D-121 — 25/10/2026 — prise.** **Flèches entre variantes, variante prise au clic (comme Framer).** Le canevas des
composants relie la variante principale à chaque variante prise par un geste : flèche verte (couleur des
composants), geste écrit dans une pastille (« Au survol », « À l'appui », « Au clic ») ; clic sur la pastille : la
variante est choisie. Cadres empilés : flèches à gauche, place réservée ; côte à côte : au-dessus. « Au clic… »
(menu « + ») crée une variante personnalisée prise au clic sur le composant : interaction de l'élément racine
`click class:is-<variante> self` (D-75), un clic la prend, le suivant revient à la principale (flèche à deux
pointes) ; le menu d'une variante personnalisée active ou retire ce passage. Survol et appui déjà créés : cochés
dans le menu. L'entrée désactivée « Au clic (étape 6.3) » disparaît.

**D-122 — 25/10/2026 — prise.** **Site ajouté en déposant son dossier sur l'accueil.** Pendant le glisser, voile sur
la fenêtre ; au dépôt, chaque dossier est ajouté : un site Revel seul s'ouvre, plusieurs sont ajoutés à la liste, un
autre site ouvre l'import (dossier examiné d'emblée). Des fichiers seuls sont refusés (message). Le glisser-déposer
de Tauri reste coupé (`dragDropEnabled: false`, nécessaire sous Windows aux glisser-déposer internes : calques,
pages, médias ; vérifié dans `tauri-utils` 2.10) ; le navigateur ne donne pas le chemin d'un dossier déposé. Sous
Windows, l'interface transmet les fichiers déposés à l'application par `chrome.webview.postMessageWithAdditionalObjects`
(message objet, ignoré par le relais des commandes) et l'application lit leurs chemins (`ICoreWebView2File`,
WebView2) ; ailleurs, ou sans chemin, le dossier est cherché par son nom dans les dossiers des sites récents, le
dossier des sites, Documents, Bureau, Téléchargements et le dossier personnel ; introuvable : message qui renvoie à
« Ouvrir un site ».

**D-123 — 25/10/2026 — prise.** **Éléments des outils réglés depuis Styles.** Défilement, carrousel et diaporama
listent leurs éléments sous leurs réglages : vignette (première image), nom (calque, texte alternatif, nom du
fichier, sinon « Élément n »), remplacer l'image, monter, descendre, dupliquer, retirer ; « Ajouter une image »
copie le dernier élément (cadre et classes compris) avec l'image choisie, ou pose une image seule
(`make::add_image_item`). Onglets : liste des onglets, « Ajouter un onglet » copie le dernier onglet et son panneau
dans chaque série, retirer emporte les panneaux de l'onglet (`tabs::add_tab`, `tabs::remove_tab`). Barre de
navigation : liens du menu, « Ajouter un lien ». Accordéon (`details`) : « Ouvert au chargement », « Un seul ouvert
à la fois » (attribut `name` commun aux volets voisins, accordéon exclusif du navigateur : Chrome et Edge 120,
Firefox 130, Safari 17.2, vérifié sur developer.chrome.com/docs/css-ui/exclusive-accordion), « Ajouter un volet ».

**D-124 — 25/10/2026 — prise.** **Revue d'ergonomie (demande du 25/10/2026).** Interrupteurs au dessin des barres
de défilement et des glissières : trait fin sombre, poignée en pilule, trait de progression jusqu'à la poignée
quand il est allumé (couleur de l'action : accent dès l'étendue standard). Dans les panneaux de l'éditeur, une
seule disposition : libellé à gauche, interrupteur au bout de la ligne. Barre du composant ouvert : rayons
concentriques (rayon des contrôles + marge), contrôles sans cadre séparés par un trait, contour neutre, nom en
vert. Mode sombre du site à côté de la langue affichée. Paramètres : présentation de chaque partie en info-bulle
(icône ⓘ du titre) au lieu d'un paragraphe (D-104 revu : aucune phrase d'explication affichée). Média : source avant
les réglages de lecture. Éléments ajoutés nommés comme leur tuile, dans la langue de l'interface (« Accordéon »
plutôt que « Accordeon »). Gestionnaire de classes : pastilles neutres comme celles du panneau de styles, bleues
pour les classes de l'élément sélectionné. Bouton « Modifier les informations du site » en petit bouton. Libellés
d'outils raccourcis pour tenir dans leur colonne (« Bandeau cookies », « Lecture auto »). Repère des éléments vides
du canevas pas posé sur un décor vide à dessein (barre de progression, élément caché aux lecteurs d'écran).
Étendue de l'accent « Étendue » : aussi l'état choisi, la classe proposée en évidence et la barre de défilement tenue.

**D-125 — 26/10/2026 — prise.** **Réglages de l'éditeur, liens de variantes à la Blender, image révélée, icônes.**
- Interrupteur allumé : la poignée devient la poignée verticale des glissières (animée), le trait de
  progression prend la couleur d'accent (contraste sans accent). Vignettes des médias : taille continue (pas de
  1 px, colonnes de largeur fixe) au lieu de paliers.
- Valeurs glissées, tapées ou aux flèches ramenées à zéro pour les propriétés qui refusent un négatif (intérieur,
  tailles, écarts, taille du texte, interligne, arrondis, épaisseurs, opacité…) ; marges extérieures, décalages,
  espacement des lettres et ombres restent libres (`revel_style::values::clamp_non_negative`).
- Réglages de Revel › Éditeur : forme des liens entre variantes (courbes, en équerre, droits) et leur bout
  (flèche ou rond) ; familles montrées par la recherche rapide (Ctrl+K). › Couleurs de l'interface : composants,
  réglé ici, hérité, traduction automatique (couleur choisie ou celle du thème). Composants violets par défaut
  (vert auparavant) ; traduction automatique en cyan pour rester distincte. › Raccourcis : une ligne par action,
  « Modifier » enregistre la combinaison suivante (Échap annule), retirer, touches par défaut, conflit signalé ;
  l'éditeur, la palette et le menu « ? » lisent les touches en vigueur (`keys.rs`).
- Liens entre variantes dessinés comme ceux des nœuds de Blender : prises rondes au bord des cadres, courbe aux
  poignées perpendiculaires au bord, contour sombre sous le trait, flèche pleine calculée sur la tangente, son
  milieu sur le trait (sources : changements de dessin des liens de Blender, lists.blender.org, 09/2022).
- « Image révélée » (panneau Ajouter, médias) : deux images superposées, la seconde montrée au survol en entier
  (fondu), jusqu'au pointeur (volet) ou dans un halo autour de lui (rayon, bord adouci) ; script du site
  (`data-revel-reveal`), au survol en entier sans script (canevas). « Média au survol » renommé « Média des
  éléments survolés » (il change avec l'élément survolé d'une liste).
- Icônes : 95 dessins de Revel (37 auparavant) rangés en onglets (flèches, interface, contact, médias, objets, création) et un
  onglet « Importées » : fichiers SVG des médias du site, « Importer » en ajoute ; dessin importé nettoyé (sans
  script, gestionnaire d'événement ni adresse `javascript:`), rempli de la couleur du texte sauf réglage du fichier.
- Mode sombre du site à gauche de la langue affichée. Défilement : un élément ajouté depuis Styles laisse l'outil
  choisi (la liste reste à l'écran).

**D-126 — 26/10/2026 — prise.** **Liens de variantes : en équerre et rond par défaut.** Forme `square` et bout
`dot` par défaut (Réglages de Revel › Éditeur ; courbes et flèche restent au choix, D-125).

**D-127 — 26/10/2026 — prise.** **Sites du compte GitHub sur un autre ordinateur.** Compte GitHub connecté :
l'accueil liste ses dépôts (propriétaire, collaborateur, membre d'une organisation ; 300 au plus, les plus
récemment envoyés d'abord) dont la racine contient `site.revel.json` (8 vérifications à la fois). Une copie locale
est retrouvée par l'adresse de son dépôt distant (sites connus et sous-dossiers du dossier des sites, sans
réseau) ; les autres sont des cartes « à télécharger » (clone dans le dossier des sites, puis ouverture).
« Synchroniser » reçoit les nouveautés de chaque copie (`versions_receive`) et rend compte (à jour, envoyés, en
attente, échecs). Dossier des sites par défaut : `Projects` à la racine du dossier de Revel.

**D-128 — 26/10/2026 — prise.** **Modèles de site par métier, inclus dans le programme.**
- Sources dans `crates/revel-core/templates/` (texte : `template.json`, feuille de style, gabarits, composants
  et pages en Nunjucks de Revel, contenus d'exemple), incluses par `build.rs` : quelques centaines de Kio, rien à
  télécharger. Un kit commun (`_kit/` : barre de navigation, pied de page, cartes, prix, FAQ, bandeau d'appel…)
  partagé par les 26 modèles (vitrine, agence, SaaS, blog, magazine, portfolio, photographe, boutique, restaurant,
  hôtel, immobilier, avocat, cabinet de santé, beauté, sport, musique, podcast, événement, association,
  formation, CV, documentation, page de liens, artisan, mariage, bientôt en ligne), chacun en français et en anglais (`[[fr||en]]`),
  clair, sombre ou selon l'appareil.
- Forme écrite : `<x-use c="carte" titre="…" x-image="expr" variant="v">emplacement</x-use>` pour un composant,
  `{{ content | safe }}` pour l'emplacement d'un gabarit ; `@@role:largeur@@` pour une photo, `@@icon:nom@@`
  pour une icône de Revel. Chaque modèle passe l'audit des réglages modifiables (essai sur les 26, deux langues).
- Photos : liens vers des photos libres d'Unsplash (licence Unsplash : usage libre, pas de compilation en
  service concurrent ; le lien direct n'est exigé que des utilisateurs de son API). Médias › « Copier dans le
  site » télécharge les images en ligne dans `media/photos/` et remplace leurs adresses partout (éléments,
  images, valeurs de composant, informations du site, contenus du CMS), une étape d'annulation ; outil MCP
  `copy_online_images`. Une image qui ne se télécharge pas garde son adresse.

**D-129 — 26/10/2026 — prise.** **Boutique : panier, commande par e-mail, liens de paiement.** Bouton
`data-revel-cart-add` (produit, prix), compteur `data-revel-cart-count`, panier `data-revel-cart="EUR"` (ligne
modèle `data-revel-cart-item`, quantités, total, champ caché rempli pour le formulaire) ; panier gardé dans le
navigateur du visiteur (`localStorage`), vidé quand la commande est envoyée (formulaire de D-54). Paiement :
liens Stripe Payment Links et boutons PayPal réglés dans le CMS ou les informations du site (vides par défaut),
aucune clé ni compte côté Revel. Panneau Ajouter : « Panier » et « Ajouter au panier ».

**D-130 — 26/10/2026 — prise.** **Galerie d'un contenu (CMS) en vignettes.** Un champ liste d'images
(`type: image` avec `list` ou `multiple`, ou liste de chemins d'images) s'affiche en vignettes : clic : remplacer,
✕ : retirer, glisser ou Alt+flèche : changer l'ordre, « Ajouter » : choix de plusieurs médias à la fois (ordre
des clics, numéros sur les vignettes choisies ; fichiers importés ajoutés au choix), fichiers déposés sur la
galerie : importés puis ajoutés ; maximum du schéma respecté. Remplace les pastilles de chemins.

**D-131 — 26/10/2026 — prise.** **Volet de la fiche réglable ; modèle des pages accessible depuis le CMS.**
Largeur du volet de la fiche (ou du champ) à la poignée de son bord gauche (320 à 1 100 px, gardée d'une session
à l'autre, double-clic : 420 px, flèches au clavier). « Modèle des pages » à côté de « Champs », dans le menu
d'une collection (clic droit) et sur une collection vide : ouvre dans l'éditeur une page de la collection qui
suit le modèle (les modifications s'appliquent à toutes ses pages sauf celles qui ont un design propre, D-28) ;
crée le modèle et l'adresse des pages au besoin, et un premier contenu « Exemple » quand la collection est vide.

**D-132 — 26/10/2026 — prise (fichier et catalogue GitHub ; pas de service).** **Templates partagés.**
Le mot « template » est gardé en français pour ces sites prêts à adapter (demande de l'utilisateur).
- *Fichier de template* (`.revel-template`, archive ZIP : `template.json`, `preview.<ext>`, `site/…`) :
  Paramètres du site › Publication › Template › « Enregistrer… ». Léger : textes, pages, composants, styles,
  contenus du CMS, icônes (SVG) et polices compris ; images du site remplacées par des images d'attente en ligne
  aux mêmes proportions (`https://placehold.co/<l>x<h>.webp`, 2 400 px au plus de côté) ; vidéos, sons et
  documents retirés (adresses vidées) ; code jamais repris (scripts, programmes, `package.json`…). Adresse
  publique, destinataire des formulaires et code personnalisé retirés du document. Ouvert dans « Nouveau site »
  › Plus › « Ouvrir un fichier » (description, auteur, licence, taille, nombre d'images à remplacer).
- *Catalogue GitHub* : dépôt public (`catalog.json` à la racine, un dossier `templates/<id>/` de la même forme
  que l'archive, tous les fichiers du site compris : images, vidéos, documents ; plus de 100 Mo par fichier :
  laissé de côté et signalé). Lecture par les fichiers bruts (`raw.githubusercontent.com/<dépôt>/HEAD/…`), sans
  limite de l'API. Envoi depuis Paramètres › Publication › Template › « Envoyer » avec le compte GitHub relié
  (D-127) : objets Git (contenus, arbre, version) poussés directement si le compte peut écrire dans le dépôt,
  sinon copie du dépôt et demande de fusion (relecture humaine). Index lu par l'API au moment de l'envoi (les
  fichiers bruts restent en cache quelques minutes). Réglages de Revel › Templates : dépôt du catalogue
  (`matv-studio/revel-templates` par défaut), nombre de templates, « Créer un catalogue sur GitHub » (dépôt public
  `revel-templates` du compte, `catalog.json` vide et présentation). Galerie : onglet « Catalogue ».
- À l'ouverture d'un template (fichier ou catalogue) : chemins vérifiés (ni remontée ni racine), taille bornée,
  fichiers de code laissés de côté, code personnalisé, éléments `<script>`, attributs `on…` et liens
  `javascript:` retirés du document ; langue et couleurs du template gardées.
- Outils pour assistants : `save_as_template`, `publish_template` (MCP).
- Service propre à Revel (comptes, notes, modération) : non retenu tant que l'usage ne le justifie pas.

**D-133 — 26/10/2026 — prise.** **Colonnes du tableau du CMS.** Épingle à côté de chaque champ de la fiche
(montré en colonne ou non) ; bouton « Colonnes » et clic droit sur l'en-tête (cocher, masquer la colonne
cliquée, largeurs automatiques). Colonnes écrites dans le schéma (`view.fields` de `.pages.yml`, comme Pages
CMS ; aucune colonne : le seul champ principal) ou, sans schéma, gardées sur cet ordinateur ; largeurs à la
poignée du bord droit de chaque en-tête (60 à 900 px, double-clic : automatique), gardées sur cet ordinateur
par collection. En-tête du CMS réduit à ses icônes quand la liste est étroite ; sous 1 050 px, la fiche passe
au-dessus du tableau.

**D-134 — 26/10/2026 — prise.** **Garder le texte d'origine (Localisation).** Coche à droite du champ d'un texte
pas encore traduit : le texte d'origine devient sa traduction (noms propres, mots identiques). Textes des
pages : traduction égale au texte ; fichiers de données : valeur écrite et clé marquée gardée
(`Locale.kept`, `data:<chemin>`, commande `MarkKept`), comptée comme traduite bien qu'égale au texte
d'origine. Une étape d'annulation.

**D-135 — 26/10/2026 — prise.** **Plan du site : contenus sous leur collection.** Les pages des contenus d'une
collection dépliée sont rangées en liste serrée sous le nœud de la collection, en retrait ; sans position
choisie, elles suivent la collection quand on la déplace ; « Ranger » fait de même.

**D-136 — 26/10/2026 — prise.** **Défilement doux refait (comme Lenis).** Position animée gardée en nombre
décimal (la fin du mouvement ne rampe plus pixel par pixel), amortissement selon le temps écoulé (même
vitesse à 60, 120 ou 144 images par seconde, images sautées comprises), limites relues à chaque image ;
pavé tactile (petits pas décimaux, déjà amortis par le système) laissé au navigateur ; liens vers une
section de la page glissés de même ; défilement venu d'ailleurs (barre, clavier) : l'animation s'arrête.

**D-137 — 26/10/2026 — prise.** **Paramètres réorganisés (suite de D-104).** Structure (pages, gabarits,
composants, collections) et environnement de publication principal : information, plus réglage ; fiche
fixée en haut à droite des paramètres avec la pastille de l'environnement (« Production », les autres en
info-bulle). Organisation : titre et « + » tant qu'elle n'est pas décrite, « Retirer » efface tout. Code
personnalisé : partie de Publication. Annuler, rétablir (boutons et Ctrl+Z) dans la barre du haut pour les
Paramètres, la Localisation et le Plan du site ; les vues relisent le document.

**D-138 — 26/10/2026 — prise.** **Variantes d'un composant, aperçu.** Canevas des composants : les règles
`:hover` et `:active` du site sont mises hors d'usage dans chaque cadre (sélecteur renommé `.rv-live-…`),
seule la variante du cadre est montrée (la principale ne réagit plus au survol, la variante « Au survol »
garde son état). « Aperçu » dans un composant ouvert : le composant seul, avec les scripts du site, pour
l'essayer avant de le placer ; ailleurs : la page affichée. Liens vers d'autres sites dans l'aperçu : chargés
dans le cadre de l'espace de travail (`frame-src` élargi) ou dans la fenêtre d'aperçu ; ceux qui s'ouvrent
dans un nouvel onglet vont au navigateur du système.

**D-139 — 26/10/2026 — prise.** **Revue d'ergonomie, deuxième passage (relecture indépendante, captures et
code).** Réglages qui demandaient de taper une valeur technique remplacés par des contrôles :
- Panier : groupe « Panier » (monnaie), « Ajouter au panier » (produit et prix, écrits ou reliés à un champ),
  compteur, rôles des parties du panier ; plus aucun attribut `data-revel-cart*` brut.
- Champ de formulaire : type, nom envoyé, obligatoire, texte d'exemple, saisie automatique, lignes visibles.
- Vidéo : source (choix parmi les vidéos), image d'attente, sans le son, chargement ; attributs gérés retirés
  des « Autres attributs ».
- Référencement : images (partage, icônes, logo) en vignettes avec choix et retrait, couleur du navigateur en
  pastille, pays en liste, réseaux en liste de liens réordonnable, image de partage plus en double.
- Informations du site : contrôle selon la valeur (image, e-mail, téléphone, lien avec « Ouvrir », texte
  long) ; nom en texte, renommé au crayon ou au double-clic.
- Listes de contenus : valeur d'un filtre et de « contient » choisie dans les choix du champ, les contenus
  reliés ou oui/non, sinon suggestions tirées des contenus ; libellés « Contenus mis en avant », « Seulement
  si le champ … contient », « Nombre avant « Voir plus » ».
- CMS : image seule en vignette (même taille que la galerie, ✕, fichier déposé) ; pastilles réordonnables
  (glisser, Alt+flèches), corrigées au double-clic, ajoutées aussi à la virgule et à la sortie du champ,
  doublon signalé ; contenus reliés réordonnables avec « n / max » ; blocs repliables (résumé), dupliqués,
  glissés, tout replier ; listes d'objets sans schéma en cartes ; nombres avec les limites du schéma ;
  adresses ouvrables ; formats proposés pour un champ texte ; adresse (slug) changée d'un contenu publié :
  redirection de l'ancienne adresse ajoutée.
- Couleurs du site : opacité réglable et visible (damier).
- Inspecteur : noms d'attributs lisibles (nom technique en info-bulle) ; lien vers une section de la page,
  un fichier ou la page d'un contenu ; classe d'une action proposée parmi celles du site, erreur dite ;
  point d'appui et position du fond en français, centre par défaut ; section Texte cachée pour les médias ;
  graisses chargées d'abord (« non chargée » pour les autres) ; délai d'une transition dans l'unité de sa
  durée ; date de publication d'une page au calendrier.
- Interface : icône « Parent » distincte de « Monter » ; icônes « Modèle des pages » et « Design propre »
  distinctes de « Panneaux » ; texte discret plus contrasté ; fenêtre gardée ouverte après une saisie (clic
  sur le fond) ; médias au clavier (Entrée : détails, Maj+Entrée : insérer) ; adresses des collections du
  plan écrites `‹slug›` ; bandeau « images en ligne » discret sous la recherche des médias, retiré une fois
  les images téléchargées.
Points restants faits ensuite (26/10/2026, soir) :
- Fenêtre étroite : colonne de droite au-dessus du canevas sous 1 000 px, celle de gauche aussi sous 760 px ;
  languette sur le bord de chaque colonne pour la replier ou la rouvrir (repliée : languette visible à toute
  largeur ; `fold_left`, `fold_right` de la disposition).
- Valeur fluide (`clamp()`, `min()`, `max()`) en pastille « Fluide 1rem → 3rem » : fenêtre Min., Idéal, Max.,
  « Valeur fixe » ; variable du site (`var(--step-5)`) en pastille « Step 5 » : liste des variables du site
  avec leur valeur, « Détacher de la variable » ; le menu des unités reste à côté (convertit en valeur fixe).
  Variables négatives nommées « Step −1 ».
- Propriété « Liste » d'un composant écrite à la main (`["a", "b"]`) : une ligne par valeur (réordonner,
  retirer, ajouter) ; liste reliée à un champ : inchangée.
- Champs du CMS liste de nombres ou de dates (schéma `number`/`date` avec `list`, ou valeurs sans schéma) : une
  ligne par valeur (champ nombre ou date), réordonnable.
- Classes du corps de page (page et modèle) : pastilles retirables, ajout avec les classes du site proposées,
  nom invalide signalé.
- Texte riche au clavier : Alt+F10 dans le texte ouvre la barre, flèches gauche/droite, Début/Fin, Entrée ou
  Espace, Échap revient au texte (`role="toolbar"`, `aria-keyshortcuts`).

**D-140 — 26/10/2026 — prise.** **Création d'un site : fenêtre compacte, galerie à la demande.** La fenêtre
« Nouveau site » reprend sa forme d'avant D-128 (nom, langue, template, couleurs, emplacement) ; le choix du
template a trois boutons : « Vide », un template inclus tiré au hasard à chaque ouverture (nom et couleurs), et
« Plus », qui élargit la fenêtre (animation de la largeur, réduite si l'utilisateur demande moins de mouvement)
sur la galerie complète : templates inclus par catégorie, catalogue GitHub, fichier de template. Un template
partagé garde sa langue et ses couleurs (champs masqués).

**D-141 — 26/10/2026 — prise.** **Interface en 15 langues.** Anglais, français, espagnol, allemand, italien,
portugais, néerlandais, polonais, turc, russe, indonésien, hindi, chinois simplifié, japonais, coréen
(`crates/revel-i18n/locales/<code>/revel.ftl`), choisies dans Réglages › Général › Langue (liste, noms dans
leur propre langue) et proposées comme langue du contenu d'un nouveau site. Traductions faites par un modèle
de langue à partir de l'anglais, vérifiées par un programme (mêmes clés, variables, attributs et sélecteurs
qu'en anglais ; essai `revel-i18n`) ; relecture par des locuteurs bienvenue. Repli sur l'anglais pour une clé
absente ; nombres groupés selon la langue. Arabe et hébreu non proposés : l'interface ne s'écrit pas encore de
droite à gauche. Guide intégré (aide) : français et anglais seulement (les autres langues lisent l'anglais).

**D-142 — 27/10/2026 — prise.** **Animations et transitions lisibles, règle des outils de l'interface.**
Chaque animation prête à l'emploi a son dessin (repères fixes et sujet qui joue le mouvement lentement, en
boucle, au survol de sa tuile ; ralenti encore si l'utilisateur demande moins de mouvement), au lieu de carrés
identiques. Transitions entre les pages : neuf effets (fondu, glissement, glisser vers le haut, pousser, zoom,
flou, volet, rideau, bascule), choisis dans des tuiles dont le dessin au repos montre l'instant du milieu de
la transition. **Règle des assistants (MCP) et du travail de Claude sur les sites** : construire seulement ce
que l'utilisateur peut régler dans l'interface de Revel, sauf demande contraire ; animations, survols,
défilement, transitions et actions passent par les interactions de Revel (attributs `data-revel-*`) et les
réglages du site, jamais par des scripts, du code ou des `@keyframes` écrits à la main.

**D-143 — 27/10/2026 — prise.** **Aperçu d'un composant comme son canevas.** L'aperçu d'un composant ouvert
n'est plus une page : le composant seul, à la largeur et au zoom de son canevas, centré, avec ses variantes et
interactions jouables ; ses liens et formulaires ne mènent nulle part (on reste dans le composant).

**D-144 — 27/10/2026 — prise.** **Modèle des pages : page interne de Revel.** « Modèle des pages » (CMS,
clic sur une collection dans Pages, ligne « Modèle des pages » sous la collection) ouvre une page qui n'existe
que dans Revel (`/__revel/template/<collection>/`) : le gabarit de la collection rendu sur un contenu d'exemple
qui réunit tous ses champs, remplis ou non (pour chaque champ la première valeur trouvée dans les contenus,
sinon une valeur d'exemple selon son type ; case cochée si un contenu l'a cochée, pour montrer les parties
conditionnelles). Aucun contenu n'est créé ; le contenu d'exemple refuse d'être écrit (ses valeurs se changent
dans le CMS). Ce qui y est modifié s'applique à toutes les pages de la collection, sauf celles qui ont un
design propre.

**D-145 — 27/10/2026 — prise.** **Classes selon le contenu en paramètres avancés.** Comme Webflow (classe de
base et classes combinées, retirées depuis le champ des classes) et Framer (classes ajoutées par du code, pas
dans l'interface), une classe ajoutée selon un champ (champ rempli, valeur d'un champ) est un réglage avancé :
« Classes selon le contenu » quitte le réglage du contenu relié pour Paramètres avancés, avec des boutons à la
charte (plus de liens soulignés) et un bouton qui la retire. Annuler (Ctrl+Z) agit aussi quand le focus est
sur une liste de choix ou une case (seuls les champs où l'on écrit gardent leur propre annulation).

**D-146 — 27/10/2026 — prise.** **Vignette des sites dans l'accueil.** Chaque site a sa vignette : couleur tirée
de son nom, couleur choisie (dix proposées ou une autre), ou image de sa page d'accueil. L'image est prise dans
le canevas (cadre caché, modern-screenshot 4.7.0, licence MIT, `crates/revel-core/vendor/`), gardée par
l'application à côté de son état (jamais dans le dossier du site), et reprise d'elle-même à l'accueil quand le
site a changé depuis.

**D-147 — 27/10/2026 — prise.** **Colonnes du CMS déplaçables, catalogues de l'utilisateur.** Colonnes du tableau
réordonnées en glissant leur en-tête ou par leur menu (« Déplacer à gauche / à droite ») ; une colonne d'image
montre une vignette (adresse au survol). Catalogues de templates : plusieurs dépôts GitHub, ajoutés depuis
Réglages › Templates (liste, état, retrait) ou depuis « Nouveau site » (bouton « + » à côté d'Inclus et
Catalogue, sans liste pour garder la fenêtre légère) ; un template s'enregistre dans le catalogue choisi.

**D-148 — 27/10/2026 — prise.** **Tailles du site et variables renommées.** Paramètres › Général › Tailles :
les variables de taille de `:root` (texte, espaces, largeurs, comme les variables de taille de Webflow) listées,
renommées partout (commande `RenameVariable`, aussi pour les couleurs : leur nom se modifie dans Couleurs),
réglées (valeur fixe, ou taille fluide : minimum et maximum d'un `clamp`), ajoutées, retirées ; aperçu « Aa »
pour celles qui servent de taille de texte. Assistants : `rename_variable`. **Règle de construction (demande de
l'utilisateur)** : une variable qui ne fait que ce que fait une classe n'a pas lieu d'être ; les tailles de texte
s'écrivent dans les classes, seules restent en variables les valeurs partagées par beaucoup de classes
différentes (marges, largeur maximale, espace entre sections, arrondi) et les couleurs.

**D-149 — 27/10/2026 — prise.** **Interrupteur clair / sombre du site.** `data-revel-switch="theme"` (Ajouter ›
« Interrupteur clair / sombre », ou le réglage « Interrupteur » d'un bouton quand le site a des couleurs pour le mode
sombre) bascule le site entre ses couleurs et leurs valeurs sombres (Paramètres › Couleurs, `@media
(prefers-color-scheme: dark)`) ; choix gardé par le navigateur, thème de l'appareil sinon. Un script placé à la fin de
`<head>` réécrit les requêtes `prefers-color-scheme` des feuilles avant le premier affichage (pas de flash) ;
`html[data-revel-scheme]`, interrupteur `aria-pressed` et `is-active` en sombre, texte `data-revel-switch-text`.

**D-150 — 27/10/2026 — prise.** **Modèle des pages : un bloc de chaque sorte ; composants réglables du dehors.** Le
contenu d'exemple du modèle des pages (D-144) met dans une liste de blocs (objets avec un champ de choix `type`) un
bloc de chaque sorte, pris dans les contenus quand l'un l'utilise, pour que chaque sorte de bloc se règle sur le
modèle. Recréation d'Alunites refaite (direction artistique « Rayonner » : papier clair et encre, corail, panneaux
arrondis, en-tête flottant, mode sombre et interrupteur), avec ces règles : animations par les interactions de Revel
(aucun `@keyframes`), blocs des pages de contenu dans le modèle (plus de composant vide), sections de page (chiffres,
clients) dans les pages pour que leurs valeurs se règlent du dehors, bandeau d'appel à contacter avec des propriétés,
cartes avec des valeurs d'exemple.

**D-151 — 27/10/2026 — prise.** **Relevé des styles ; aucun code CSS pour l'utilisateur.** Demande : « ALWAYS use
Revel's built-in tools » (effet de survol des expertises d'Alunites écrit en CSS, introuvable pour un non-technicien).
Le relevé « tout éditable » (D-90) couvre aussi la feuille du site (`revel_edit::styleaudit`, outil MCP
`check_site`, `revel audit`) : sélecteur que le panneau Styles ne règle pas (`style_selector` : `.a > .b`, `.a .b`,
`::before`, listes), requête média autre qu'un point de rupture du site ou règle de classe dans le mode sombre
(`style_media`), bloc de code (`style_code` : `@keyframes`), variable locale ou valeur qui n'est ni une couleur ni
une taille du site (`style_variable`), propriété sans réglage dans le panneau (`style_property`). Formes admises :
classe ou combinaison, balise proposée, un état du panneau (`:hover`, `[aria-current="page"]`, `[open]`…), élément
dans un élément actif, ouvert, éteint, en pause ou dans une vidéo lancée (nouvel état « Dans une vidéo lancée »),
variante de composant (`.carte:hover .titre`), balise dans une classe (`.prose p`, `.titre strong` : nouveau choix
du panneau, pastille « Tous les … dans .classe » et liste « Contenu… »), couleurs et tailles du site, `@font-face`,
et la seule fondation `*, *::before, *::after { box-sizing: border-box }`. Ajoutés au panneau : puces d'une liste
(`list-style-type`). Outils qui portent leurs propres styles (piste d'un carrousel : défilement de côté, accroche,
barre masquée ; image responsive : `<picture>` sans boîte ; lecteur vidéo qui remplit sa couverture) écrits par
Revel avec une priorité nulle, jamais par l'utilisateur. Recréation d'Alunites réécrite à zéro point : survols
d'enfants en variantes de composant (« Ligne d'expertise », cartes, « Membre de l'équipe », logos clients), points
et voiles en éléments, menu et interrupteur par l'état « Actif », couleurs d'expertise en classes selon un champ,
mode sombre seulement par les couleurs du site.

**D-152 — 27/10/2026 — prise.** **Plan du site : ouverture sur les pages ; double-clic sur une adresse.** Les
parties communes sont masquées à l'ouverture du plan (bouton « Parties communes »). Dans « Médias et réseaux », un
double-clic sur une adresse (ou sur un de ses endroits) ouvre le contenu du CMS où elle sert, sinon l'élément de la
page dans l'éditeur ; bouton « Modifier le contenu » sur chaque endroit d'un contenu.

**D-153 — 27/10/2026 — prise, revue le même jour.** **Texte ajusté à l'écran : une unité de la taille du texte.**
Styles › Texte › Taille propose, après px, rem, em, %, vw et vh (ajoutée), l'unité « Écran » : `--revel-fit: 42` dans
la règle, la part de la hauteur de l'écran que le texte occupe, sans déborder en largeur (entre 12 et 600 px). La
taille écrite (`font-size`) reste celle d'un navigateur sans script ; choisir une autre unité retire `--revel-fit`.
Le script lit les règles de la page (points de rupture compris), recalcule quand la fenêtre, les polices ou le texte
changent, et tourne dans le canevas. Première version (réglage séparé « Ajuster à la hauteur de l'écran »,
`data-revel-fit`) retirée sur la remarque de Stéphane : une demande proche d'un réglage existant s'y intègre.
Accroche des projets d'Alunites : lignes en phrase, la première en gras, 42 % de l'écran ; point final retiré.

**D-154 — 27/10/2026 — prise.** **Élément à glisser.** Interactions › Glisser : `data-revel-drag` (sens libre,
horizontal ou vertical, rester dans son conteneur, élan, collisions entre les éléments à glisser d'un même
conteneur), par la propriété `translate` (s'ajoute à la transformation réglée dans Styles). L'import de Framer
reprend les éléments `drag` (élan par défaut comme Framer, limites si `dragConstraints`) : Framer n'a pas de
collisions, l'option se coche dans le panneau.

**D-155 — 27/10/2026 — prise le 28/10 (suite : D-157, D-158).** **Imports sans code ni variables.** Première partie, sans changer le rendu :
couleurs de Framer (`--token-…`) en couleurs du site, variables de Framer jamais déclarées remplacées par leur
valeur de repli, propriétés sans effet retirées, remplissage des images de fond en classes. Reste : les styles de
fonctionnement de Framer (texte, composants, styles écrits sur les éléments, portée de page) ne se traduisent
fidèlement qu'à partir des styles calculés de chaque élément à chaque point de rupture ; outil à construire
(« styles calculés » dans un cadre caché, comme les vignettes D-146) pour tous les imports.

**D-156 — 27/10/2026 — prise (demande de l'utilisateur).** **Réglages en double ou en code, revue.** Après D-153,
revue des outils déjà faits : un réglage proche d'un réglage existant s'y intègre, un outil ne laisse ni code ni
variable que le panneau ne sait pas régler.
- Barre de progression (D-110) : plus de variable `--revel-progress` ni de `calc()` ; le script écrit la largeur du
  remplissage (enfants de la barre), dont la règle garde `width: 0%` (sans script), la couleur, la hauteur,
  l'arrondi et la transition, réglables dans Styles. Anciennes barres : la largeur écrite par le script l'emporte.
- Couleurs des blocs prêts à l'emploi et du bouton « Ajouter au panier » (`var(--bloc-accent, #2f6fed)`) : à
  l'insertion, les variables que le site n'a pas sont déclarées dans `:root` avec leur valeur de secours ; elles
  se règlent dans Paramètres › Couleurs et dans le sélecteur de couleur. Site vide et blog : `--bloc-accent`
  déclarée d'emblée.
- Vérifiés sans changement : « Afficher d'abord » (à côté de « Nombre de contenus » de la liste), maçonnerie (bouton
  de la grille), interrupteur clair / sombre (réglage « Interrupteur » d'un bouton), puces (Styles › Texte),
  tailles et couleurs du site (Paramètres, choisies dans les champs de Styles), défilement doux (Paramètres ›
  Général). L'image révélée et le média des éléments survolés n'ont pas d'équivalent dans Interactions › Survol
  (volet, halo, média d'une liste) : outils gardés.

**D-157 — 28/10/2026 — prise.** **Mise à plat des styles (suite de D-155).** Les règles que le panneau Styles ne
sait pas régler (sélecteurs descendants, listes, pseudo-éléments, raccourcis à variables, styles écrits sur les
éléments, variables locales) sont reportées sur les classes des éléments : pour chaque élément et chaque point de
rupture, cascade calculée (spécificité, ordre, `!important`, héritage des variables) avant et après, la différence
écrite sur une classe combinée ou sur la classe propre de l'élément quand tous ceux qui la portent la partagent ;
pseudo-éléments avec contenu changés en éléments (`<span class="X__after" aria-hidden="true">`) ; règles mortes
retirées ; points de rupture déduits quand le site n'en a pas. Une seule commande (annulable). Faite à chaque import
et à la création d'un site depuis un modèle ; bouton clé dans Classes, outil MCP `make_styles_editable`, commande
`revel flatten`. Panneau complété pour ce qui restait en code : contour (`outline`), clics (`pointer-events`), mots
longs et coupure des mots, états `:focus`, `:disabled`, `.is-open`, `.is-empty`, balises `svg`, `pre`, `code`.
Une règle reste telle quelle quand le report changerait le rendu dans un cas que les pages rendues ne montrent
pas : élément du site non rendu qui pourrait la porter (branche d'une condition ; chaque page écrite une à une est
rendue, les contenus d'une collection sont échantillonnés, les composants sont rendus seuls pour le contenu par
défaut de leurs emplacements), attribut `hidden` posé par un script, variable lue par une animation
(`@keyframes`), règle sous une condition non calculée (survol d'un ancêtre, mouvement réduit) que le report
écraserait ; une règle d'état simple (`.filtre.is-active`) est reportée après la base. Les classes d'état
(`is-*`) ne reçoivent jamais de report.
Vérifié : modèles 1 472 → 0 réglages en code, 162 pages identiques au pixel près ; site Framer d'essai 833 → 56
(restent variantes de bordure de Framer, survol d'un lien dans un texte riche, deux sélecteurs de structure),
aucun écart de style calculé à 1440, 1000, 600 et 390 px ; site cobaye Alunites reconstruit 273 → 0, importé
comme site HTML 282 → 166, aucun écart de style calculé ni de capture (11 et 9 pages, 1280 et 375 px).
Complément (28/10, site matv) : panneau complété par la barre de défilement (`scrollbar-width`, affichée quand le
débordement défile), le soulignement (couleur, épaisseur, écart, affichés quand le texte est souligné) et le masque
(`mask-image`, mêmes calques que le fond). Une classe d'image écrite par une expression (`"a" ~ …`) reçoit le
report dans l'expression. Une règle `.contexte .racine` qui vise la racine d'un composant selon l'endroit où il est
utilisé devient une classe de l'utilisation (`racine--contexte`, posée sur chaque utilisation sous le contexte) ;
règle gardée si le composant est utilisé dans un autre composant. Vérifié : site matv recréé, réglages en code à 0,
132 combinaisons pages × largeurs comparées au rendu d'avant, 23 écarts tous expliqués (puce d'un `summary` sans
marque, minutage des apparitions).

**D-158 — 28/10/2026 — prise.** **Horloge et toile de pixels.** Outils relevés sur le site Framer d'essai
(composants de code « Time & Date » et « Pixel Trail »), sans équivalent dans Revel.
- Horloge : `data-revel-clock` sur un texte (Ajouter › Horloge) ; heure, date ou les deux, secondes, 12 heures,
  jour de la semaine, année, mois (en toutes lettres, abrégé, chiffre, masqué), réglés dans l'inspecteur ; texte
  remplacé chaque seconde par `Intl.DateTimeFormat` dans la langue de la page ; chiffres de même largeur
  (`:where`, réglables dans Styles).
- Toile de pixels : `data-revel-pixels` sur un bloc (Ajouter › Toile de pixels) ; traînée qui s'efface ou dessin qui
  reste, taille d'un pixel, couleur (une couleur du site suit le thème), seulement en appuyant, durée de la
  traînée ; un `canvas` ajouté par le script, pixels tracés sans trou entre deux positions du pointeur.
- Import de Framer : propriétés lues dans le code de l'instance, conteneur reconnu par sa classe ; le texte caché
  rendu par Framer devient l'horloge, les calques de la toile sont retirés.

**D-159 — 29/10/2026 — prise (retours de l'utilisateur sur la 0.7.2).** **Éléments ajoutés simples, panneau
Styles allégé.** Demandes : un élément du panneau Ajouter doit être rapide à poser et à régler, sans classe
créée d'office ni structure exposée dans les calques ; panneau Styles sans menus inutiles.
- Éléments ajoutés sans classe : leur allure vient de Revel (`data-revel-base="grid|switch|…"`, styles écrits à
  priorité nulle, `crates/revel-render/src/toolcss.rs`, couleurs tirées du texte : clair comme sombre) ; les
  classes de l'utilisateur passent devant. Aucune règle écrite dans la feuille du site (sauf blocs prêts à
  l'emploi). Choix de l'utilisateur parmi deux : « élément unique » plutôt que « composant de la bibliothèque ».
- Éléments construits fermés (`data-revel-widget`) : interrupteur, lecteur vidéo, filtres, onglets,
  accordéon, carrousel, diaporama, défilement, barre de navigation, langues, barre de progression, image
  révélée, consentement. Un seul calque ; leurs parties (`data-revel-part`) n'ont pas de ligne, un clic dessus
  choisit l'élément ; le contenu accueilli (diapositive, panneau, lien du menu) reste un calque. Réglages à
  l'élément : textes de ses parties, images (couverture), commande et textes d'un interrupteur, boutons et
  valeurs des filtres. « Décomposer » (réglages, menus contextuels) retire `data-revel-widget`.
- Un seul « Interrupteur » (thème par défaut ; Commande : thème, musique, sons). Texte riche et Liste
  retirés du panneau Ajouter (choix de l'utilisateur ; l'éditeur de texte des contenus garde les listes).
  Recherche dans le panneau Ajouter.
- Panneau Styles : classes en tête, « + » seul tant que l'élément n'en a pas (balises et balises d'un texte
  riche dans son menu) ; liste d'Effets sur une ligne avec son « + » ; cadrage et point d'appui en X / Y
  chiffrés ; valeur montrée pendant un glisser ; information du site repliée en une icône ; rôles dans un
  élément interactif rangés dans Paramètres avancés.
- Grille « Aligner » : Alt+clic étire, double-clic ou Ctrl+clic répartit (comme la boîte d'alignement de
  Webflow, vérifié sur help.webflow.com). Image sans fichier montrée dans le canevas (cadre et icône).
- Complément (04/11, choix de l'utilisateur) : la première valeur réglée sur un élément sans classe crée une
  classe, comme Webflow, sans double inutile : nom d'après la nature de l'élément (`grid`, `button`… pour un
  élément ajouté) ; un nom porté par aucun élément et sans réglage est repris (sa règle vide remplie) plutôt
  que numéroté.
- Prévu avec l'intégration 3D (04/11, demande de l'utilisateur) : le catalogue `matv-studio/revel-templates`
  n'est alimenté que par l'utilisateur ; l'envoi de template par défaut vers ce dépôt est retiré de Revel.

**D-160 — 04/11/2026 — prise (demande de l'utilisateur).** **3D, étape 1 : visualiseur de modèles et
élément Marmoset.** Demande : un visualiseur de modèles 3D du niveau de Marmoset Viewer, et plus tard des
outils de sites 3D interactifs (onglet à part : scène, états, événements, caméra, listes de collection en 3D,
pages dans la scène, solution de repli ; accord de principe de l'utilisateur, phases suivantes). Étape 1 :
- Formats : GLB et glTF seulement, sans conversion (choix de l'utilisateur : « trop de risques d'erreurs »).
  OBJ et FBX écartés. Les extensions de compression courantes sont lues : Draco, Meshopt, KTX2 (Basis).
- Moteur : three.js 0.186.1 (MIT ; la 0.187.0 du 30/10 est trop récente), assemblé une fois par
  `tools/three/` (esbuild) et rangé dans `crates/revel-render/vendor/three/` avec sa licence. Hébergé avec
  le site, aucun service extérieur. Script `/assets/revel-model.js` ajouté aux seules pages qui ont un
  modèle ; moteur (`revel-three.js`) chargé quand le modèle approche de l'écran ; décodeurs Draco et Basis
  (`/assets/revel-3d/`) téléchargés seulement si le fichier en a besoin.
- Caméra et tailles d'écran : cadrage calculé à chaque changement de taille d'après la sphère englobante
  du modèle et les proportions de l'élément. Le plus étroit des deux angles de vue décide : en portrait, la
  largeur, donc le modèle n'est jamais coupé sur téléphone. Réglages relatifs (angle horizontal, angle
  vertical, marge, point visé décalé, champ de vision), donc valables à toutes les tailles ; la forme de
  l'élément se règle dans Styles par point de rupture, comme une vidéo. Limites de rotation verticale et de
  zoom. Caméra du fichier utilisable. « Prendre la vue actuelle » dans le canevas.
- Gestes : glissement vertical d'un doigt réservé au défilement de la page (`touch-action: pan-y`),
  horizontal pour tourner, pincer pour zoomer ; molette sans effet tant qu'on n'a pas cliqué dans le
  modèle (ou avec Ctrl), pour ne pas bloquer le défilement. Mouvement réduit demandé : pas de rotation
  automatique.
- Rendu : éclairage studio intégré (RoomEnvironment) ou fichier HDR, exposition, ombre au sol, fond
  transparent (couleur réglée dans Styles) ou environnement visible. Animations du fichier (choix, boucle).
  Vues : rendu, couleur de base, normales, rugosité, métal, filaire.
- Économie : calcul seulement quand quelque chose bouge ; pause hors de l'écran ; finesse plafonnée à 2 ;
  image d'attente (poster) jusqu'au chargement et si WebGL manque ; contexte WebGL perdu : rechargé.
- Élément fermé (D-159) `data-revel-base="model"` ; ses commandes visibles (plein écran, recadrer, vues)
  sont des boutons parties de l'élément (`data-model-do`), donc stylables et retirables.
- Élément Marmoset `data-revel-base="mview"` : fichier `.mview` (exporté de Marmoset Toolbag, format
  propriétaire) lu par le lecteur officiel chargé depuis `https://viewer.marmoset.co/main/marmoset.js`
  (non copié dans Revel). Taille suivie (`resize`), démarrage automatique ou au clic, fond transparent,
  interface masquée. Licence du lecteur non vérifiée (choix de l'utilisateur : « on veut des éléments
  fonctionnels, le reste on verra après »). Le serveur doit répondre aux requêtes partielles
  (`Accept-Ranges`, `Content-Length`) pour la vignette ; sinon un dégradé la remplace.
- Sources : [three.js — npm](https://www.npmjs.com/package/three),
  [Marmoset Viewer — embedding](https://marmoset.co/?p=45453).
- Complément (05/11/2026) : fichiers 3D dans les templates partagés (D-132) : `glb`, `gltf`, `mview`
  et les fichiers d'un glTF (`bin`, `ktx2`, `hdr`) sont des fichiers lourds : gardés dans le catalogue,
  retirés du fichier léger. Avant, `mview`, `bin`, `ktx2` et `hdr` étaient traités comme du code, donc
  jamais repris : un glTF partagé perdait ses données. Import depuis un lien : `ktx2`, `hdr` et `mview`
  cités dans un script sont copiés comme `glb`, `gltf` et `bin`. Scène Marmoset essayée en vrai
  (05/11/2026, navigateur réel, lecteur officiel, scène publique de Marmoset) : chargement, démarrage
  automatique, taille suivie, fond transparent, démontage et remontage.
- Un seul outil (05/11/2026, demande de l'utilisateur : même utilité, seul le fichier diffère) : la tuile
  « Scène Marmoset » est retirée du panneau Ajouter ; « Modèle 3D » a dans ses réglages un choix « Format »
  (GLB / glTF ou Marmoset). Changer de format garde l'élément (identifiant, calque, classes, autres
  attributs), retire les réglages et boutons de l'ancien format, pose ceux du nouveau, garde le fichier s'il
  convient et reprend l'image d'attente (`revel_edit::make::model_kind_command`, une étape annulable).
  Choisir un fichier de l'autre format (`.mview` dans un modèle GLB, ou l'inverse) change aussi le format.
  Le site produit garde deux lecteurs (`data-model`, `data-mview`) : formats et moteurs différents. Le
  préréglage `mview` reste pour les assistants (MCP). Calque nommé « Modèle 3D » dans les deux formats.
- Parité des deux formats (05/11/2026, règle de l'utilisateur : la parité est très importante, à faire sans
  demander) : mêmes textes pour les mêmes réglages (« Fichier », « Image d'attente », mêmes info-bulles),
  mêmes groupes (Rendu › « Fond visible », Boutons › « Commandes du lecteur »), même sens (fond visible
  allumé = fond de la scène, éteint = transparent) et même valeur par défaut (fond transparent : le
  préréglage `mview` pose `data-mview-transparent`). Clés propres à Marmoset retirées (`mview-file`,
  `mview-thumbnail`, `mview-transparent`, `mview-noui`), `mview-controls` ajoutée.

**D-161 — 04/11/2026 — prise (erreur relevée par l'utilisateur).** **GitHub Pages : activation après
l'envoi.** En publiant alunites.com vers `Alunites/alunites-revel`, la branche `gh-pages` a été envoyée, puis
l'activation de GitHub Pages (`POST /repos/{dépôt}/pages`) a répondu 500 sans détail ; Revel affichait
« GitHub returned an error (500): ». Une erreur 500 vide à cet appel est déjà connue comme passagère côté
GitHub ([discussion GitHub Community](https://github.community/t/cannot-enable-github-pages-via-api-blank-500-error/124406)).
- Après une erreur 5xx : relecture de l'état de Pages (il a pu être créé malgré l'erreur), puis un nouvel
  essai, une fois (`Client::ensure_pages`).
- Échec persistant : message propre à cette étape (le site est sur la branche, seule l'activation manque),
  statut et détail, adresse des réglages Pages du dépôt pour l'activer à la main.
- Appels et préfixe d'adresse du site faits avec le nom exact du dépôt rendu par GitHub (casse, dépôt
  renommé), pas avec le nom tapé.

**D-162 — 06/11/2026 — prise (choix de l'utilisateur : « comme Framer »).** **Styles propres à une
utilisation de composant.** Une utilisation de composant n'avait dans Styles que ses propriétés : impossible
de la placer (en-tête fixe sur une page, carte absolue) sans changer le composant partout. Comme dans Framer
(position, taille d'une instance), le panneau Styles complet s'affiche pour une utilisation : ses classes
sont posées sur l'élément racine du composant pour cette utilisation seule (attribut `class` de `Node::Use`,
déjà fusionné par `patched_body`) ; la première valeur crée la classe `<composant>-instance`. Le composant
garde son allure ; l'utilisation règle sa place. Écartée : la façon de Webflow (racine du composant seule,
bloc autour pour un cas particulier). Essai éditeur : position relative posée sur l'utilisation « Header »
puis annulée.

**D-163 — 06/11/2026 — prise.** **Contenu précédent ou suivant d'une collection.** Filtres
`getPreviousCollectionItem` et `getNextCollectionItem` (noms d'Eleventy, pour les imports) :
`(collections.x | getNextCollectionItem(page, true)).url`, `true` : du dernier au premier. Dans un modèle des
pages, la destination d'un lien propose « Contenu précédent » et « Contenu suivant » (Page › Contenu
voisin) ; le relevé les reconnaît (`collection::neighbor_link`).

**D-164 — 06/11/2026 — prise (demande de l'utilisateur, tableau validé).** **Template « Galerie 3D ».**
Accueil en grille filtrable (catégories), collection `models` (nom, catégorie, fichier 3D, image d'attente,
description, auteur, licence, source), modèle des pages en visionneuse plein écran : modèle absolu sur toute
la scène (`100dvh`), barre du haut, description et crédit, flèches vers les modèles voisins (D-163), menu
des modèles à droite (panneau qui monte du bas sur téléphone) ouvert et fermé par l'action « Ajouter une
classe » (`click class:is-open .viewer`). Page À propos avec les crédits tirés du CMS.
- Modèles : Khronos glTF Sample Assets, licences relevées dans `metadata.json` de chaque dossier : CC0
  (ToyCar, SheenChair, Lantern, BoomBox, WaterBottle, Corset) et CC BY 4.0 (AnisotropyBarnLamp,
  MaterialsVariantsShoe, DiffuseTransmissionPlant) ; écartés : DamagedHelmet (part CC BY-NC 4.0), ceux qui
  portent des logos (CarConcept, ChronographWatch…), ceux sans GLB. Liés en ligne (les fichiers des
  templates sont du texte) ; « Copier dans le site » copie aussi `.glb` et `.mview` (dossier `3d/`).
- Images d'attente : rendus faits avec le lecteur de Revel (fond transparent), dans
  `docs/templates/gallery3d/`, liés comme les photos des autres templates (`photos.json`).
- Action « Ajouter une classe » : son état suit désormais la classe de l'élément visé, pour que plusieurs
  boutons (« Modèles », la croix du menu) pilotent le même panneau.
- Écart avec le tableau : pas de compteur « 03 / 09 » (pas d'écriture du rang d'un contenu dans l'interface).
- La visionneuse montre la barre des vues (dont Calques, D-165) et le compteur triangles / sommets.

**D-165 — 06/11/2026 — prise (demande de l'utilisateur : « la fonction la plus importante de mviewer »).**
**Modèle 3D : vue « Calques » et compteur de triangles et de sommets.** Vue `layers` ajoutée aux vues du
modèle (`data-model-view`, barre des vues) : six bandes obliques (rendu, normales, couleur, métal, rugosité,
filaire), chacune dessinée seulement dans sa bande par le tampon de pochoir de WebGL (bandes écrites dans le
pochoir, puis une passe du modèle par vue), traits et noms des bandes par-dessus (nœuds `data-rv-layers`), à
la manière du lecteur de Marmoset Toolbag. Le fond de la scène n'est pas montré dans cette vue (il couvrirait
toutes les bandes). Réglage « Triangles et sommets » (`data-model-stats`, groupe Rendu) : nombre de triangles
et de sommets de toutes les mailles du modèle (`index.count / 3` ou `position.count / 3` ; sommets :
`position.count`), en bas à gauche (`[data-rv-stats]`, styles de l'outil, réglables par le site). Noms des
bandes et du compteur : textes de l'interface (mêmes clés que les réglages, 15 langues) insérés dans
`revel-model.js` à la génération, choisis d'après la langue de la page.
- En suspens : pente des bandes et ordre des vues fixes (pas de réglage) ; pas de glissement des traits à la
  souris comme dans Marmoset.

**D-166 — 12/11/2026 — prise (demande de l'utilisateur, capture à l'appui).**
**Calques à la manière de Marmoset Viewer, compteur en icônes, template « Galerie 3D » refait.**
Lecteur officiel relu (`https://viewer.marmoset.co/main/marmoset.js`, objet `StripData`) : un bouton « Layer
Views » ouvre les bandes ; cinq bandes égales (pente 0,25) couvrent l'écran, le rendu reste à droite ; un
appui sur une bande la montre seule en plein écran, un nouvel appui revient aux bandes ; les traits glissent
par décroissance exponentielle (demi-vie 50 ms). Repris dans `model.js` (remplace le dessin de D-165) :
- bandes égales sur toute la largeur (Normales, Couleur, Métal, Rugosité, Filaire, puis le rendu), traits
  `cuts` animés (demi-vie 70 ms, `prefers-reduced-motion` : sans animation) à l'ouverture, au choix d'une
  vue et à la fermeture ;
- choix à l'écran : appui bref sans glisser sur une bande (ou son nom, bouton atteignable au clavier) ; vue
  seule : nom en pastille en haut, appui : retour aux bandes ; bande du rendu : calques fermés ; glisser
  tourne toujours le modèle ; survol : nom de la bande en évidence, pointeur en main ;
- bouton `data-model-do="layers"` (icône de plans empilés, Styles › Boutons › Calques) ; `view:layers` garde
  le même effet ; `data-model-shown` donne la vue montrée ; `--model-layers-top` règle la hauteur des noms ;
- allure des traits, noms et compteur dans les styles de l'outil (`toolcss.rs`), plus dans le script.
Compteur : une icône et un nombre par mesure (triangle ; vertex : angle et point), nom en info-bulle et pour
les lecteurs d'écran ; « Vertex » en français (demande), « Triangles et vertex » dans Styles.
Template « Galerie 3D » : visionneuse sans nom de modèle ni barre des vues ; boutons en verre dépoli (retour,
menu), modèle sur toute la scène, description sur deux lignes et crédit en bas à gauche, flèches au centre,
boutons Calques / Recadrer / Plein écran en bas à droite ; voile cliquable derrière le menu ; accueil :
cartes sans cadre (vignette carrée sur fond de scène, nom et catégorie dessous), surtitre retiré.
- En suspens : ordre des bandes fixe ; traits non déplaçables à la souris.

**D-167 — 12/11/2026 — prise (besoin du site Alunites Groupe : projets et articles de deux autres sites).**
**Flux RSS 2.0 (et Atom) reliés à une collection.** Avant : Revel produisait des flux RSS (`feeds`) mais
n'en lisait pas. Ajout : `settings.feed_sources` (collection, adresse, origine, dernière reprise), commande
`SetFeedSources`, lecture dans `revel-import::feed` (roxmltree, déjà présent) : RSS 2.0 (titre, lien, guid,
pubDate, description, category, enclosure, `content:encoded`, `dc:creator`, `media:content`,
`media:thumbnail`), RSS 1.0 et Atom 1.0. Chaque article devient un contenu Markdown de la collection : `title`,
`date`, `summary` (300 caractères), `image`, `tags`, `author`, `link`, `guid`, `source`, texte en HTML.
Reprise (`Core::feed_sync`, commande `feed_sync`) : contenus reconnus par `guid` et mis à jour (champs du
flux remplacés, champs ajoutés par l'auteur gardés), nouveaux ajoutés, rien de supprimé ; plusieurs flux par
collection, repris dans l'ordre, un article de même titre déjà repris d'un autre flux écarté (doublon
espècehumaine / alunites.com) ; un flux illisible est signalé, les autres repris ; une seule étape
d'annulation. Interface : bouton « Flux » de l'en-tête d'une collection (bleu quand un flux est relié), menu
(origine, adresse, ajouter, retirer, Reprendre, date, bilan). Images laissées à leur adresse d'origine
(« Copier dans le site » des médias en ligne les rapatrie).
- En suspens : reprise automatique (à la génération ou planifiée) ; flux JSON Feed ; correspondance de
  champs réglable ; les sites sources doivent exposer un flux (espece-humaine.com, superficiel.film à
  vérifier).

**D-168 — 12/11/2026 — prise (besoin du site Alunites Groupe, maquette et contexte fournis).** **Outils
ajoutés pour refaire le site dans Revel, chacun rangé là où l'interface règle déjà la même chose.**
- **Thème par défaut du site** (`settings.default_scheme`, commande `SetDefaultScheme`) : Paramètres ›
  Couleurs, sous les couleurs sombres (Appareil, Clair, Sombre). Le script de l'interrupteur de thème
  (D-149) l'applique tant que le visiteur n'a pas choisi ; il est posé sur toutes les pages dès qu'un thème
  par défaut est réglé, même sans interrupteur.
- **Élément montré en mode clair ou sombre seulement** (`data-revel-scheme-only`, réglage « Thème » sous
  l'identifiant) : pour ce qui n'est pas une couleur (deux versions d'un logo, icône lune ou soleil). Le
  mode sombre de Revel reste fait de couleurs ; cet outil couvre le reste sans requête média écrite.
  Dans le canevas, la règle est posée par `canvas.js` avec celle des calques masqués (l'attribut est
  écrit sur place, sans redessiner la page).
- **Animations** (panneau Interactions) : « Dévoiler une image » (apparition `unveil` : cadre ouvert par le
  haut, image resserrée de 1,08 à 1) ; « Suivre » (survol `follow` : un halo glisse vers le
  pointeur dans son parent, puis revient ; réglage « À l'opposé » pour un second halo en miroir) ; « Gouttes »
  (défilement `drops` : barre en trois pastilles fondues par un filtre SVG, celle du centre se détache en
  descendant la page et revient en remontant ; couleur reprise du fond de la barre, relue au changement de
  thème ; durée réglable, retour à 0,7 fois ; sans mouvement si le visiteur le demande).
- **Action « Afficher la suite d'une liste »** (`more`) : un bouton fait avancer une liste à affichage
  progressif (`data-revel-more`), qui n'avance alors plus au défilement ; le bouton disparaît quand tout
  est montré. L'affichage progressif tient compte des filtres de liste (seuls les éléments gardés comptent,
  la liste repart de ses premiers éléments à chaque filtre).
- **Dates en français** : premier jour du mois écrit « 1er ».
- **Corrections trouvées en route** : la mise à plat des styles prenait le morceau écrit d'une classe
  calculée (`dot--{{ source }}` -> `.dot--`) pour une classe, d'où des règles sans effet ; dans une autre
  langue, les liens vers les pages d'un site importé n'étaient pas préfixés quand le dossier de la page
  était aussi un dossier copié.
- Pourquoi : la maquette fait foi pour les animations ; sans ces outils, il fallait du code écrit dans le
  site (script, requêtes média), que l'interface ne règle pas.
- En suspens : adresses traduites par langue (`/en/the-group`) ; destinataire du formulaire selon un
  choix du visiteur ; reprise planifiée des flux ; réglages des seuils de « Gouttes » dans l'interface
  (`down`, `up`, `in` lus dans l'attribut).

**D-169 — 13/11/2026 — prise (demande de l'utilisateur : contrôle qualité complet de l'interface, relevé
`revel audit` du site Alunites Groupe à prendre en compte).** **Chaque écran, menu et outil contrôlé contre
la charte et les règles d'ergonomie ; écarts corrigés ; deux outils ajoutés au panneau Styles.**
Méthode : règles de la charte traduites en contrôles mesurés dans la page (libellé et info-bulle des
boutons-icônes, curseur, cibles de clic, nom accessible des champs, hauteurs et tailles de texte hors
échelle, capitales, phrase visible, contraste AA, défilement horizontal, rôle des choix exclusifs), passés
sur environ 200 états de l'interface en thème sombre ; fermeture de chaque menu essayée par Échap, clic
dans le canevas et clic dans un panneau ; chaque commande du panneau Styles essayée (effet dans le canevas,
annulation) ; temps de réaction mesurés.
- **Menus et canevas.** Le canevas est un autre document : ses appuis et son Échap ne parvenaient pas à la
  fenêtre, donc les menus ouverts (options de l'aperçu, profil, options d'un panneau, menu contextuel)
  restaient ouverts. Le canevas signale tout appui (`down`) et Échap hors saisie et hors geste (`escape`) ;
  l'interface les rejoue sur la fenêtre, où les menus écoutent. Échap ferme d'abord le menu ou la fenêtre
  ouverts, sinon retire la sélection (décidé par l'interface, plus par le canevas). Échap ferme aussi le
  menu de l'aperçu, le menu d'un panneau et les volets (paramètres de page, de modèle, détails d'un média :
  le champ en cours de saisie est quitté d'abord).
- **Contraste (thème sombre).** `--text-muted` #a3a3a3 et `--text-faint` #969696 (4,5:1 au moins sur toutes
  les surfaces où ils servent) ; texte discret d'une ligne active, survolée ou sélectionnée : un cran plus
  clair. Pourquoi : compteurs, en-têtes de tableau, pastilles et légendes passaient sous AA (3,4 à 4,4:1).
- **Curseur.** Plus de main sur les commandes (charte : outil de bureau) ; la main reste aux liens.
- **Échelles.** Tailles de texte, rayons et durées écrits en dur ramenés aux jetons (10 et 10,5 px → 11 px) ;
  parties des Paramètres et tri des médias à 24 px ; cibles de clic des en-têtes triables et des compteurs
  du site portées à 24 px sans changer la mise en page ; libellés du menu contextuel jamais coupés.
- **Aucune phrase d'explication visible.** Aides et états vides : un mot court et l'explication en
  info-bulle (ⓘ) — panneaux Styles et Interactions sans sélection, média au survol, onglets, modèle des
  pages, flux RSS, remplacement d'un média, plan du site, champs déduits du CMS, menus du site, formulaire
  d'hébergement (`icons::info_tip`, `empty_tip`, `note_tip`).
- **Noms accessibles.** Commandes d'une ligne du panneau Styles groupées sous le nom de la ligne ; valeur
  d'une couleur du site, déclaration libre, type de point de rupture, champ de texte, cases de position du
  fond : nommés. Titre complet en info-bulle des titres coupés du CMS.
- **Bordure.** Régler une épaisseur de bordure ou de contour sans trait pose le trait plein avec elle :
  avant, rien ne se dessinait.
- **Parties « avant » et « après »** (`::before`, `::after`) : deux états du menu « État » du panneau Styles,
  proposés pour tout élément qui peut en porter. Le champ « Contenu » règle leur texte (`content`) ; un
  contenu vide est posé d'office au premier réglage, sans quoi la partie n'existe pas. Le canevas donne les
  valeurs calculées de la partie réglée.
- **État d'un ancêtre** (`.carte:hover .titre`, `.lien:focus-visible .fleche`) : le menu « État » propose
  « Survol de .classe » pour chaque ancêtre qui porte une classe (cinq au plus, du plus proche au plus
  lointain) et « Focus sur .classe » si l'ancêtre se prend au clavier. Plus besoin d'en faire un composant
  avec une variante. Le relevé des styles reconnaît ces formes (`ANCESTOR_STATES`).
- Relevé du site Alunites Groupe : sélecteurs écrits 66 → 26, propriétés sans réglage 31 → 25.
En suspens : variables locales (`--c` posée sur une classe, `rgba(var(--c), …)`), propriétés sans commande
(`-webkit-line-clamp`, `transition-delay`, `isolation`, `writing-mode`, compteurs, `-webkit-text-stroke`),
`:nth-child`, balises `li` et `span` dans une classe ; côté page : classe calculée d'après un champ, attribut
conditionnel (`target` si lien externe), liste filtrée par référence. L'opacité d'un élément animé à
l'apparition reste masquée dans le canevas par l'animation jouée (valeur réglée correcte).

**D-170 — 13/11/2026 — prise (demande de l'utilisateur, site Alunites Groupe : une adresse de réception par
type de besoin choisi dans le formulaire de contact).** **Formulaires : destinataire selon la valeur d'un
champ.** `FormSettings.route_field` (nom du champ, attribut `name`) et `routes` (valeur → adresse) ;
Paramètres › Formulaires : « Destinataire selon le champ », une ligne par valeur. Le script PHP produit à la
génération compare la valeur reçue (sans tenir compte de la casse) et envoie à l'adresse de la ligne, sinon à
l'adresse générale ; lignes incomplètes ignorées ; les adresses restent côté serveur, rien n'est choisi par
le visiteur en dehors de la liste. Le script est produit dès qu'une adresse existe (générale ou d'une ligne).
Pourquoi : jusque-là une seule adresse par site, donc un tri à la main des messages d'un groupe à plusieurs
marques. Conseil : donner aux choix de la liste une valeur stable (`value`), la même dans toutes les
langues, sinon la valeur envoyée change avec la traduction. En suspens : choisir le champ et ses valeurs
dans une liste tirée des formulaires du site plutôt que de les écrire ; essai de bout en bout de l'écran.

**D-171 — 13/11/2026 — prise (demande de l'utilisateur, site Alunites Groupe : « si un outil Revel permet de
faire on remplace par l'outil de Revel, sinon on crée l'outil ; pas de restes d'import »).** **Outils ajoutés pour
ce que `revel audit` relevait encore sans commande.** Listes de collection : « À partir du n° » (`Query.offset`,
écrit `(liste)[o:o+n]` ou `[o:]`, relu à l'ouverture), pour une liste qui reprend après une mise en avant
(« la une » puis les suivantes). Panneau Styles : Typographie › nombre de lignes affichées
(`-webkit-line-clamp` et ses propriétés liées posées ensemble), sens d'écriture (`writing-mode`), contour du
texte (épaisseur et couleur, `-webkit-text-stroke-*`) ; Effets › perspective, isolation, apparence native et
redimensionnement (ces deux derniers montrés pour les champs et boutons, ou s'ils ont une valeur) ; Fonds ›
découpe du fond (`background-clip`, montrée avec une image ou une valeur) ; Disposition (grille) › hauteur des
lignes ajoutées (`grid-auto-rows`). Le relevé connaît ces propriétés (`PANEL_PROPS`). Pourquoi : ces règles
venaient de maquettes importées et ne se réglaient qu'en code ; d'autres sites en auront besoin. Le reste du
relevé du site Alunites Groupe a été repris avec les outils existants (carrousel `data-revel-slider` pour une
galerie défilante, ligne et colonne de grille à la place de `grid-area`, défilement doux du site à la place de
`scroll-behavior`, couleurs de marque en couleurs du site) : relevé à zéro. En suspens : le relevé ne reconnaît
pas, dans une boucle dans une boucle, une valeur reliée à la variable de la boucle extérieure (contourné sur le
site par la variable de la boucle intérieure) ; essai de bout en bout des nouvelles commandes.

**D-172 — 16/11/2026 — prise (demandes de l'utilisateur du 16/11).** **Outils regroupés, « Sauvegarde », retour
au CMS et outils créés pour les gabarits du site Alunites Groupe.**
- Panneau Ajouter : un outil par famille, la fonction se choisit dans ses réglages (« outils > sous-outils »).
  Vidéo : Vidéo, Lecteur (couverture et bouton), Média au survol ; Image : Image, Révélation. Commande
  `ConvertPreset` : élément remplacé par la fonction choisie en gardant ses médias (vidéo, image, couverture).
  Carrousel et diaporama : un seul outil (les réglages du carrousel couvrent défilement auto et boucle).
- « Versions » devient « Sauvegarde » (Paramètres) : mise en ligne sur GitHub d'abord, historique en plus.
  Emplacements « Local » (ancien « Seul ») et « Dépôt en ligne » ; « Dossier partagé » et le tableau comparatif
  retirés de l'interface (un site déjà en dossier partagé le garde). Dépôt choisi : première synchronisation
  lancée aussitôt (fichiers envoyés).
- Menus de la barre du haut au premier plan (panneau du profil caché par la bande des réglages du site).
- Modèle des pages ouvert depuis le CMS : l'onglet CMS reste actif, bouton de retour en haut à gauche (nom de
  la collection) ; position du CMS (collection, fiche ouverte, champs, recherche, tri, défilement du tableau)
  gardée d'un onglet à l'autre, pour le site ouvert. Le retour s'oublie en quittant l'éditeur ou le modèle.
- Outils créés pour les gabarits validés (et tout site) : Sommaire (`data-revel-toc` : titres choisis d'une
  zone, partie lue surlignée) ; actions « Copier l'adresse de la page » (`copy`, état « Lien copié »
  `.is-copied`, aussi « dans un bouton qui a copié ») et « Partager la page » (`share:<réseau>` : LinkedIn, X,
  Facebook, WhatsApp, e-mail, partage du téléphone) ; animation de texte « Palettes » (`flap`, tableau
  d'affichage de gare, `from=` : texte de départ) ; écriture « Temps de lecture » d'un texte riche ou des blocs
  (`reading_time`, 200 mots par minute) ; nombre d'éléments encore cachés écrit dans le bouton « Afficher
  plus » (`data-revel-rest`).
- Dossier du site sur le serveur (SFTP) : la connexion d'essai liste aussi les autres dossiers de l'accueil
  (sous-domaine en « multisite » : `groupe/`) ; dossier écrit par l'utilisateur gardé (il était remplacé par
  la suggestion) ; sous-domaine `x.domaine.fr` : dossier `x/` proposé ; champ avec propositions au lieu
  d'une liste fermée ; Paramètres › Publication : bouton « Lire les dossiers du serveur » pour changer de
  dossier.
- Images responsives : les `<source>` d'un `<picture>` sans boîte, sinon dans une rangée ou une grille avec
  écart chacune comptait comme un élément vide et décalait l'image.
Pourquoi : demandes de l'utilisateur ; les gabarits validés demandaient ces fonctions, faites en outils plutôt
qu'en code propre au site (relevé `revel audit` du site à zéro). En suspens : le partage ne montre pas de
compteur ; le Sommaire ne suit qu'une zone par page.

**D-173 — 16/11/2026 — prise (retour de l'utilisateur : « les boutons de la galerie ne fonctionnent pas », « déplacer
les images en clic-glisser à la souris »).** **Carrousel : images responsives en diapositives, glisser à la souris.**
Une image responsive placée directement dans la piste est un `<picture>` sans boîte (`display:contents`) : sa
position lue était toujours 0, les flèches et le compteur ne bougeaient pas. Le script lit maintenant la position
de l'image contenue, et l'accroche du défilement vaut aussi pour elle. Glisser à la souris : la piste suit le
pointeur puis s'arrête sur la diapositive la plus proche (la suivante après un geste court) ; curseur « main » ;
le clic qui termine un glisser n'ouvre pas de lien. Référence : le carrousel et le diaporama de Framer se
glissent à la souris ; un élément `display:contents` n'a pas de boîte (CSSOM View). Version 0.7.7.

**D-07 — à confirmer avant la première version publique.** Identifiant d'application `app.revel-studio.desktop`
(`tauri.conf.json`). Il détermine les dossiers de données et les mises à jour : le changer après diffusion casse la continuité.
Basé sur le domaine `revel-studio.app`, libre mais non enregistré.

**D-08 — close le 23/11/2026 par D-192** (licence MIT).

**D-09 — ouverte (étape 6 ; voie Windows choisie le 23/11/2026 : SignPath Foundation, D-192).** Signature de code : certificat Windows (sinon avertissement SmartScreen), compte Apple Developer (sinon blocage Gatekeeper).
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

**D-174 — 16/11/2026 — prise (retour de l'utilisateur : « je ne suis pas censé voir toute la structure interne
des outils dans les calques », « trois sections texte qui ne font aucun sens », « tu as utilisé une collection CMS
pour les marques alors qu'il aurait fallu un composant » ; propositions validées sur une toile Design).**
**Carrousel refait en outil fermé, composant relié au CMS.**
- **Carrousel** (`Preset::Carousel`, `Preset::Slideshow`) : racine `data-revel-slider` fermée (`data-revel-widget`),
  piste d'images (`Node::Image` vides : icône d'image du canevas, plus de texte « Diapositive 1 »), commandes
  toutes présentes et montrées ou cachées par l'attribut `hidden` (titre, flèches, compteur, points). Une ligne
  dans les calques, marquée de l'icône de l'outil ; seules les diapositives se déplient ; « Décomposer » (clic
  droit) retire `data-revel-widget`. Panneau Styles (`apps/revel-ui/src/carousel.rs`) : fonction (Images,
  Contenu, Mosaïque), images en vignettes (ajout, retrait, ordre) ou champ d'images du contenu, affichage
  (nombre visible, écart, proportions, arrondi ; colonnes et bouton « +N » en mosaïque), commandes
  (interrupteurs, icônes remplaçables, texte du titre, position dessous, dessus ou sur les côtés), défilement
  (glisser, lecture automatique, boucle). Le champ « Texte » générique n'est plus montré pour l'outil. Réglages
  portés par des variables (`--rv-show`, `--rv-gap`, `--rv-ratio`, `--rv-radius`, `--rv-cols`) et des attributs
  (`data-revel-controls`, `data-revel-layout`), style de base dans `toolcss.rs` (spécificité nulle). Script :
  mosaïque, glisser à la souris, compteur recalculé quand la piste prend sa taille (`ResizeObserver`).
  Références comparées : Slider de Webflow (réglages dans le panneau, flèches et points en éléments), Slideshow
  et Carousel de Framer (flèches, points, glisser en options), composants à variantes de Figma.
- **Propriété « Contenu »** (`PropKind::Entry`, `PropDef.collection`) : un composant relié à une collection lit
  les champs du contenu (`<prop>.data.<champ>`, `<prop>.url`) dans tous ses éléments ; l'utilisation choisit le
  contenu : « Collection » (nom), puis un menu « Contenu » (premier de la collection, contenu de la liste qui
  l'entoure, ou un contenu nommé : `collections.<c> | by_slug("…")`). Vérifié par `revel audit`.
- **Site Alunites Groupe** : galeries, mosaïques, sommaire et lecteur vidéo des gabarits sont les outils fermés
  de Revel ; cartes de projet, d'actualité et de marque sont des composants reliés au CMS (les textes des
  marques sont passés dans la collection Marques, variante « Inversée ») ; composant « Bouton » (texte, lien,
  variante « Contour »). `revel audit` : 0.
- En suspens : les autres outils (onglets, accordéon, filtres, barre de navigation) n'ont pas encore leur
  panneau refait sur ce modèle ; titres et sections d'introduction du site restent des éléments écrits (leur
  mot accentué est un élément dans le titre, qu'une propriété « Texte » ne porte pas) ; templates de Revel à
  refaire avec ces outils ; dépôt des templates à créer.

**D-175 — 16/11/2026 — prise (demandes de l'utilisateur avant la 0.7.8).** **Réglages par défaut, rouage,
mise à jour en un clic, élément ajouté montré dans les calques.**
- Nouvelle installation : interface en anglais, accent bleu `#3D7EFF`, étendue « Étendue » (`full`). Les réglages
  déjà enregistrés ne changent pas.
- Barre du haut : les réglages de Revel sont un rouage, à l'accueil et à droite de « Settings » dans un site ;
  les vues du site restent affichées quand les réglages de Revel sont ouverts (un clic y ramène) ; le bouton
  « Revenir à [site] » est retiré.
- Mise à jour disponible : la pastille ouvre la fenêtre « Installer Revel x et redémarrer ? » au lieu des
  réglages.
- Élément ajouté : le panneau « Calques » passe devant et montre l'élément (réglage « Montrer l'élément ajouté
  dans les calques », Réglages › Général, activé par défaut ; `AppSettings.reveal_inserted`).
- Panneaux : un groupe retiré d'une colonne rend sa hauteur aux autres (`Layout::fill`, aussi à la relecture de
  la disposition enregistrée).

**D-176 — 17/11/2026 — prise (demande de l'utilisateur : « on ne passe pas en 0.7.8 tant que tous les outils ne
sont pas revus » ; tableau « Les autres outils » validé sur la toile Design).** **Panneaux des outils fermés :
onglets, accordéon, barre de navigation, textes nommés.**
- **Règle commune** : un outil fermé (`data-revel-widget`) ne montre plus le champ « Texte » générique ; ses
  textes se règlent dans son panneau, dans la liste de ses parties ou dans un groupe « Textes » où chaque texte
  porte le nom de sa partie (bouton, libellé, titre…). Chaque outil fermé porte son icône sur sa ligne des
  calques (défilement, interrupteur, bandeau cookies, progression, langues, modèle 3D, horloge compris).
- **Liste des parties** (`apps/revel-ui/src/toolparts.rs`, `PartsList`) : une ligne par partie répétée,
  renommée sur place (le texte de la partie, dans la langue en cours), montée, descendue, retirée ; « Ajouter »
  copie la dernière ; second champ facultatif (attribut).
- **Onglets** : liste des onglets (un onglet se déplace avec ses panneaux, `tabs::move_tab`), « Ouvert au
  départ » (`start=<n>` dans `data-revel-tabs`, lu par le script).
- **Accordéon** : l'élément ajouté est un groupe fermé (`data-revel-accordion`, nature `accordion-group`) de
  trois volets `details` (titre avec icône, contenu en calque ordinaire). Panneau : volets listés, « Ouvert au
  départ », « Un seul ouvert à la fois » (attribut `name` des volets), icône d'ouverture (la même partout,
  remplaçable) et son côté (`data-revel-icon="start"`). Les `details` seuls déjà en place (blocs FAQ) gardent
  leurs réglages.
- **Barre de navigation** : marque (texte, ou logo choisi dans les médias), « Menu replié » (`data-revel-collapse` :
  tablette 991 px, téléphone 767 px par défaut, petit téléphone 479 px, toujours, jamais), bouton du menu en
  icône remplaçable (`aria-label` pour les lecteurs d'écran).
- **Sélecteur de langue** : réglage « Afficher » (nom ou code de la langue) à la place de « Liste de
  collection ». **Horloge** : outil fermé nommé « Horloge ». **Vidéo** : les choix de « Fonction » se partagent
  la largeur du panneau ; **lecteur vidéo** : image de couverture, icône du bouton de lecture (remplaçable) et
  nom du bouton réglés au panneau.
- Élément répété d'un outil (`data-revel-item`, volet d'un accordéon) : calque ordinaire dont les parties
  restent celles de l'outil (cachées des calques, stylées par la nature de l'outil).
- Références comparées : réglages « Tabs settings » de Webflow (ajout, retrait, onglet par défaut), réglages
  « Navbar settings » de Webflow (largeur à laquelle le bouton du menu apparaît), accordéons de Framer (éléments
  listés, un seul ouvert, ouvert par défaut, icône chevron ou plus).
- En suspens : montrer le menu ouvert dans le canevas (Webflow : « Open menu : Show ») ; filtres de liste :
  ordre des boutons.

**D-177 — 17/11/2026 — prise (demandes de l'utilisateur : « continue sur TOUS les outils qui n'ont pas encore
été revus », « retire le bouton Décomposer du panneau style », « rajoute un petit Classes au-dessus du champ »,
règles de construction du MCP « comme un professionnel du métier »).** **Revue des outils restants, panneau
Styles allégé, règles du MCP.**
- **Panneau Styles** : plus de bouton « Décomposer » (il reste au clic droit, dans les calques et le canevas) ;
  le champ des classes porte un libellé « Classes ».
- **Défilement** : les éléments ajoutés sont des images vides (icône d'image du canevas), plus de texte
  « Élément 1 ». **Filtres de liste** : boutons dans la liste commune des parties (texte, valeur, ordre).
  **Bandeau cookies** : textes des boutons nommés « Accepter » et « Refuser ». **Barre de navigation** :
  « Montrer le menu » ouvre le menu dans le canevas le temps du réglage (message `tool-preview` du canevas,
  rien n'est écrit dans le site). **Blocs « Formulaire » et « Panier »** : les réglages du formulaire et du
  panier sont montrés sur le bloc. **Code intégré** : nommé dans les calques.
- Outils relus sans changement de réglages (panneau jugé complet face à Webflow et Framer) : barre de
  progression (ce qu'elle suit ; épaisseur et couleur par Taille et Couleur du texte), sommaire (niveaux de
  titres, zone), horloge, toile de pixels, Lottie, modèle 3D, liste de collection, vidéo, grille, colonnes,
  bouton « Ajouter au panier », interrupteur.
- **Règles du MCP** (`crates/revel-mcp/src/tools.rs`, `INSTRUCTIONS`) : partie « Work like a professional web
  designer » (ordre de travail : variables du site, composants pour tout ce qui se répète, CMS pour les
  contenus qui ont leurs pages ou servent sur plusieurs pages, outils de Revel jamais refaits à la main,
  calques propres, points de rupture, accessibilité, vérification finale) ; propriété « Contenu » des
  composants et réglages des outils (carrousel, accordéon, onglets, barre de navigation) décrits.
- En suspens, à décider par l'utilisateur : appliquer ces règles à l'import (variables, composants et classes
  détectés automatiquement) ; mode simple sans classes visibles (comme Framer) avec un réglage pour les montrer.


**D-178 — 17/11/2026 — prise (demandes de l'utilisateur : « on passe en mode simple par défaut, et activer les
classes dans les settings », « re-analyse un site sans classes pour les ajouter de façon logique », « pour
l'import, ajoute les variables, composants, couleurs et taille de site », « même incertain il est préférable
d'utiliser un outil de Revel facilement modifiable », « ajoute le rework de l'import dans la 0.7.8 », « un
setting pour désactiver le warning de publication quand un media dépasse 20 MB »).** **Mode simple, import
remis en forme, médias lourds.**
- **Mode simple par défaut** (réglages de Revel › Général › « Classes », coupé) : le champ des classes, le
  compteur d'éléments et le panneau « Classes » ne sont pas montrés. Le modèle ne change pas (une règle par
  classe) ; un élément réglé qui partage toutes ses classes avec d'autres reçoit une classe à lui (les autres ne
  bougent pas) ; un élément seul sur ses classes les garde. Pourquoi : les classes en double venaient du réglage
  d'éléments qui partageaient une classe sans que l'utilisateur le voie ; Framer ne montre pas de classes.
- **Classes activées** : à l'activation du réglage, le site ouvert (et ceux ouverts ensuite dans la session)
  reçoit une fois la proposition de regrouper les classes de mêmes valeurs (`revel_edit::classes::organize` :
  une seule classe gardée, renommée sans numéro quand le nom est libre) ; le panneau Classes garde le bouton
  « Regrouper » avec le nombre de classes concernées. Jamais proposé à la simple ouverture d'un site.
- **Import remis en forme** (`revel_import::refine`, appelé par `rebuild_for_ui` après la mise à plat des
  styles, pour tous les imports ; commande `revel refine <dossier>` pour un site déjà importé) :
  1. *Outils reconnus* : accordéon (`details` qui se suivent, blocs en-tête + panneau nommés `accordion`,
     `faq`…, `aria-expanded`, listes `dl`), onglets (rôles `tab` et `tabpanel`, composants de Webflow et de
     Bootstrap, noms `tabs`), carrousel (Swiper, Slick, Splide, Glide, Flickity, Owl, Embla, Keen, Webflow,
     noms `carousel`/`slider`, piste `scroll-snap-type` ou `overflow-x` sans nom), barre de navigation
     (`header`, `nav` ou bloc nommé avec marque, liens et bouton de menu). L'outil est celui du panneau
     « Ajouter » (même forme), avec le contenu d'origine (textes, liens, diapositives, panneaux et leurs
     classes). L'outil garde l'allure du site : classes de la racine, des volets, des en-têtes, des onglets, de
     leur liste, des panneaux, du menu, de ses liens et du bouton de menu gardées ; attribut `data-revel-own`
     sur la racine, qui coupe l'allure propre de Revel (`toolcss` : règles `:not([data-revel-own])`, bordures,
     remplissages, graisses) et laisse ses règles de fonctionnement. Ce que le site faisait lui-même par ses
     styles est retiré de la feuille, l'outil s'en charge : état caché des panneaux (`display: none`,
     `max-height: 0`…), `display` du menu et du bouton de menu ; l'allure du menu ouvert du site
     (`.menu.open`) devient celle du menu replié ; la classe d'état de l'onglet choisi est renommée
     `is-active` (posée par l'outil). Repli du menu : le plus petit de ceux de l'outil qui couvre celui du
     site. Classes des bibliothèques de carrousel retirées ; flèches, points et icônes sont ceux de Revel.
     Garde-fou : rien n'est refait si du contenu ne serait pas repris (texte ou média hors des parties, sous-
     menus) ; la construction est laissée et nommée dans le bilan. `revel import --no-tools`, `revel refine
     --no-tools` ou `REVEL_NO_TOOLS=1` : étape passée.
  2. *Classes regroupées* (même fonction que le bouton « Regrouper »).
  3. *Composants* : éléments de même forme (balises, classes, attributs) répétés au moins trois fois, ou deux
     fois à partir de huit éléments ; ce qui change devient une propriété (texte, lien, image, autres attributs
     en texte) avec la valeur de chaque utilisation ; une liste d'éléments répétés n'est pas un composant, ses
     éléments le sont ; un bloc répété deux fois ne compte que sur deux pages ; douze composants au plus (ceux
     qui retirent le plus de répétitions), un bloc avant ses parties ; pas de composant dans un composant ni
     dans un outil pour l'instant. Passe sautée si le site a déjà des composants (Eleventy, Framer).
  4. *Variables du site* : couleurs employées au moins deux fois (seize au plus) écrites dans `:root` et lues
     par `var()`, nommées par leur rôle (`--texte`, `--fond` : couleurs de `body`) ou leur teinte (`--bleu`,
     `--gris-clair`) ; tailles de texte employées au moins trois fois (`--texte-18`, `--texte-fluide-48`) ;
     largeur maximale la plus employée (`--largeur-site`). Une couleur déjà écrite dans `:root` est reprise.
  Les passes 2 à 4 ne changent pas le rendu : page produite identique avant et après (essais), styles calculés
  comparés sur le site de référence dans la CI (étape « Import HTML statique », `revel refine --no-tools`).
  Bilan ajouté à celui de l'import.
- **Regroupement des classes, correction** : deux classes de mêmes valeurs ne sont regroupées que si leurs
  règles sont du même côté (avant ou après) de chaque autre classe des mêmes éléments qui règle une même
  famille de propriétés (`classes::RuleOrder`). Sans cela le regroupement changeait la règle qui l'emporte
  (constaté sur le site de référence : un en-tête passait de `block` à `flex`).
- **Médias lourds** (réglages de Revel › Général › « Signaler les médias lourds », activé par défaut) : coupé,
  le contrôle « images de plus de 1 Mo » n'est plus fait avant la publication (ni montré, ni compté dans les
  points qui arrêtent la publication). Le code n'a pas de seuil à 20 Mo ; la limite de taille par fichier d'un
  hébergeur (25 Mio Cloudflare Pages, 100 Mio GitHub Pages) reste bloquante, le fichier serait refusé.
- En suspens : case « Refaire avec les outils de Revel » dans la fenêtre d'import (aujourd'hui toujours fait
  dans l'interface) ; composants imbriqués ; collections du CMS déduites des listes de cartes ; espacements en
  variables ; éléments facultatifs d'un composant (propriété « affiché ou masqué ») pour réunir les cartes
  de formes voisines ; scripts du site qui visaient une construction refaite (une erreur de script peut
  arrêter le reste du script : à vérifier après import) ; **templates** :
  ils s'appuient sur les classes visibles, à revoir avec le mode simple (tâche du rework des templates).

**D-179 — 17/11/2026 — prise (demandes de l'utilisateur : composants reliés au CMS dont les champs ne se
saisissent pas à la main ; en-tête, pied, cartes, en-têtes de page et boutons en composants ; couleurs des
marques introuvables ; conditions illisibles et non sélectionnables ; raccourci « . » du pavé numérique ; flux
RSS de Webflow sans `https://`).** Réponses de l'utilisateur : champs CMS « masqués, sous un menu déroulant
"Champs CMS", visibles grisés une fois dépliés, avec une icône pour remplacer la valeur » ; collection Marques
gardée, avec sa couleur ; en-tête et pied dans le gabarit, à rendre accessible ; « . » amène à l'écran sans zoom.
- **Champs CMS d'une utilisation** (Styles, sous « Propriétés ») : les textes lus dans le contenu choisi
  (propriété « Contenu », ou contenu relié) ne sont plus dans les textes libres ; groupe replié « Champs CMS »
  (état gardé pendant la session), champs grisés en lecture seule, un par valeur lue (sortie répétée montrée
  une fois, éléments d'une liste réunis). Icône crayon : valeur remplacée **sur cette utilisation** — une
  propriété texte sans défaut est ajoutée au composant, chaque sortie du champ devient `{{ p or champ }}`, la
  valeur est écrite sur l'utilisation (`overrides::set_override`). Icône retour : valeur rendue au contenu.
  Ces propriétés de remplacement ne figurent pas dans « Propriétés ». Le rendu relie désormais au contenu les
  textes d'une utilisation dont le contenu est choisi par filtre (`collections.x | by_slug("…")`) ou laissé au
  contenu par défaut, et relie `p or champ` au remplacement s'il est écrit, sinon au champ.
- **Gabarits** : section « Gabarits » sous l'arbre des pages (gabarits partagés, hors modèles de collection,
  avec le nombre de pages) ; un clic ouvre le gabarit seul dans le canevas (`/__revel/layout/<id>/`, page
  fictive sans contenu), où en-tête et pied se sélectionnent et se modifient pour toutes les pages.
  Identifiant avec dossier (`layouts/base`) accepté. Recommandation suivie (pratique des outils du domaine :
  Webflow, Framer) : en-tête et pied dans le gabarit, et composants s'ils servent ailleurs.
- **Conditions** : panneau « Affichage conditionnel » lisible (menus nommés, info-bulles), le code replié sous
  « Code ». Commande « Aperçu » Auto / Si / Sinon : force la branche montrée dans le canevas (`rvif=<id>.1`,
  `EditorOptions::forced`, rien n'est écrit dans le site) pour sélectionner les éléments de la branche cachée ;
  remise à Auto en changeant de page. Attributs conditionnels (`when`) réglés par le même éditeur de condition ;
  champs d'une boucle de collection englobante proposés (« Collection › Champ »).
- **Couleurs du site** : toutes les règles `:root` lues (la dernière valeur l'emporte), modification dans la
  règle qui déclare la variable ; les couleurs des marques déclarées dans un second bloc apparaissent.
- **Touche « . » du pavé numérique** (raccourci réglable « Amener à l'écran ») : selon la zone sous la souris,
  calque sélectionné (Calques), média (Médias), composant (Composants), sinon élément amené à l'écran dans le
  canevas, sans zoom.
- **Flux RSS** : adresse sans schéma complétée en `https://` (et montrée complétée). Le champ `source` des
  contenus repris, s'il est une référence Pages CMS vers une collection (Marques), reçoit le slug du contenu
  correspondant (par slug ou titre). Cartes : lien `{{ item.data.link or item.url }}`, nouvel onglet quand le
  lien est externe.
- **Règle des assistants (MCP)** : tout bouton, lien texte, carte, titre de section, lien de navigation et bloc
  répété devient un composant ; couleur propre à une utilisation par variable de style reliée ; en-tête et pied
  dans le gabarit ; composants reliés : choisir le contenu, remplacer au besoin sur l'utilisation.
- **Corrections** : aplatissement — une classe d'utilisation n'écrase plus la première quand le même sélecteur
  existe sous une autre requête média ; audit — `x or y` (deux valeurs reliées, ou y littéral), attributs
  conditionnels et variables de style reliées acceptés ; i18n — clé `cms-fields` en double (essai ajouté :
  aucune clé en double).
- **Site Alunites** (hors dépôt, `post.py`) : carte d'accueil d'une marque, en-tête de page, bouton d'action,
  lien texte et lien de section en composants ; couleur de chaque marque dans la collection Marques ; rendu
  identique au pixel sur sept pages à 1440 et 390 px, audit 0.
- En suspens : remplacement des images d'un contenu (seuls les textes) ; libellé d'un élément de liste (« Mot »
  au lieu du nom du champ) ; contenus repris d'un flux qui reçoivent encore une page locale ; Superficiel doit
  republier son site Webflow pour que l'adresse du flux réponde (404 le 17/11).

**D-180 — 17/11/2026 — prise (demandes de l'utilisateur : recadrer une image par double-clic dans le canevas
ou sur sa vignette, zoom et position dans Styles pour les outils ; icônes du site claire et sombre sur une
ligne, pour l'icône du site, l'icône PNG et l'icône d'écran d'accueil ; flux des marques et doublons ;
construction 0.7.9).** Réponse de l'utilisateur : « cadrage sans perte ».
- **Recadrage sans perte** : attribut `data-revel-crop="x y zoom [arrondi]"` (point de cadrage en %, zoom 1
  à 4, arrondi des coins en px). Le fichier n'est pas modifié. Le site produit reçoit un style écrit sur
  l'image (`revel_model::Crop::style`) :
  - `object-fit: cover` et `object-position` ;
  - zoom par `scale` autour du point de cadrage ;
  - `clip-path: inset(…)`, calculé pour couper l'image à sa boîte sans changer la mise en page, coins
    arrondis gardés.
  Ce calcul fonctionne dans tous les navigateurs : `object-view-box` n'existe pas dans Firefox ni dans
  Safari (vérifié sur caniuse). Le style s'ajoute à un `style` écrit. L'attribut ne reste que dans le canevas.
- **Fenêtre de recadrage** :
  - ouverture : double-clic sur une image dans le canevas (hors utilisation de composant : le double-clic y
    ouvre toujours le composant), double-clic sur la vignette du panneau Styles, ou sur celle d'un élément
    d'outil (carrousel, défilement…) ;
  - réglage : glisser pour placer, molette ou réglette pour le zoom, X et Y en % ;
  - « Réinitialiser » rend l'image entière, centrée et sans zoom (l'attribut est retiré).
  - Panneau Styles d'une image : vignette cadrée, zoom, X et Y.
  - Canevas : style recalculé sur place quand l'attribut change.
- **Icônes du site** (Paramètres › Référencement) : icône du site, icône PNG et icône d'écran d'accueil ont
  chacune deux choix sur une ligne, Clair et Sombre (`favicon_dark`, `favicon_png_dark`, `touch_icon_dark`).
  - Rendu : `<link … media="(prefers-color-scheme: light|dark)">`, la sombre après la claire.
  - Chrome lit `media` pour l'icône (annonce Blink). iOS ne change pas l'icône d'écran d'accueil selon le
    thème : la claire y reste.
  - `Command::SetSeoSettings` porte désormais ses réglages dans une `Box` (taille de la commande).
- **Flux** : un contenu écrit à la main, sans identifiant de flux et avec le même titre qu'un article du flux,
  est remplacé par la version du flux. Même fichier, donc même adresse ; ses autres champs sont gardés.
  Auparavant la version du flux était ignorée comme doublon.
- **Version 0.7.9**.
- En suspens :
  - recadrage d'une image `Node::Image` (gabarits Eleventy) et d'une image dans une utilisation de composant
    (faite dans le canevas du composant) ;
  - aperçu en direct dans le canevas pendant le réglage (la fenêtre a son propre aperçu).

**D-181 — 17/11/2026 soir — prise (retours de l'utilisateur sur la 0.7.9 : flux espècehumaine refusé, dates
des flux, « 61 adresses sans page », panneau Styles surchargé, logo de l'installateur).** Lot 1 (corrections) ;
le lot 2 (composants façon Framer, outil « Champ CMS », outils En-tête et Pied à la place des gabarits, outil
Conditions complet) est décidé après questions à l'utilisateur.
- **Flux, lecture universelle** (`revel_import::feed::fetch`) :
  - adresse sans schéma complétée ; essai avec et sans « www. » ; flux annoncé par une page
    (`<link rel="alternate">`) suivi ;
  - sans flux (réponse 404 : RSS non publié), la collection est lue dans ses pages : liens de l'accueil et
    des pages qu'il lie vers `/<collection>/<nom>`, titre, résumé, image et date pris dans les balises de
    partage de chaque page (60 pages au plus) ; fin de titre commune (« | Nom du site ») retirée ;
  - commande `revel feed-read <adresse>` pour vérifier une source.
- **Date d'un contenu repris** : la date du flux est aussi gardée dans `feed_date` ; une date corrigée à la
  main (différente de `feed_date`) n'est plus écrasée. Constat : Webflow écrit dans `pubDate` la date de
  création de l'élément (13/08/2024 pour les cinq projets de Superficiel), Revel la lisait correctement.
- **Plan du site** : une ancienne adresse redirigée n'est plus comptée ni dessinée comme une adresse sans page
  (nœud discret « ancienne adresse », lien vers elle non cassé).
- **Panneau Styles** : « Modifier le composant », « Détacher » et la pastille du composant passent en icônes à
  droite du nom (barre du calque) ; le rôle « Filtres de liste » n'est plus proposé sur une utilisation de
  composant ; le menu d'état n'est montré qu'avec les classes activées (seul moyen de régler un survol ou un
  pseudo-élément d'une classe ; le retirer aussi là supprimerait cette fonction) ; propriété « Contenu » d'une
  carte dans une liste de collection : verrouillée (la liste fournit le contenu).
- **Réglages de Revel** : « Classes », « Signaler les médias lourds » et « Montrer l'élément ajouté dans les
  calques » dans un panneau « Options » de la partie Éditeur.
- **Installateur Windows** : images de l'assistant (`windows/installer-*.bmp`) refaites avec le logo actuel.
- **Site Alunites** (retouché sur place, `patch181.py`, le site n'est plus régénéré) : couleurs réunies (18
  variables retirées, `--trait-fort` ajoutée), icônes des prix nettes et grises hors survol, lueur de la
  couleur du prix au survol, rail des fiches sous la barre de navigation (code ajouté du site : dépend de
  l'état de la barre), carte d'accueil d'une marque sans numéro ni puce et sans variante « Accueil ».
- En suspens : lot 2 ; traduction de `prop-entry-list-help` (anglais hors français) ; essai e2e du plan du site
  pour les anciennes adresses.

**D-182 — 17/11/2026 20h50 — prise, à coder (lot 2 des retours sur la 0.7.9 ; réponses de l'utilisateur).**
Objectif : composants « au plus près de Framer », sans code visible. Version 0.7.10 construite après ce lot.
- **Composants** :
  - propriétés simples (texte, image, lien, couleur, interrupteur) réglées par utilisation ;
  - un texte du composant se modifie dans le composant, jamais dans une donnée : le double-clic dans le
    canevas sur un texte relié au CMS ne doit plus changer le contenu sans le dire ;
  - variantes : seulement celles créées par l'utilisateur, plus le survol.
- **Cartes de marque** (site Alunites) : textes, logo, lien et couleur en propriétés de chaque carte ; la
  collection « Marques » est supprimée (« pas logique dans la création d'un site »). À revoir avec elle : le
  champ « source » des projets et des articles (référence vers Marques), les filtres par marque, le pied.
- **Outil « Champ CMS »** : élément posé dans un composant ou une page, réglé par deux menus (collection,
  champ), pour un texte, une image ou un lien ; plus aucune expression `{{ … }}` montrée dans le panneau.
- **Liste de collection** : réglage de ce qui est montré — automatique (tri, nombre, filtre) ou choix manuel des
  contenus, dans l'ordre voulu. La carte d'une liste reste une seule carte modèle.
- **En-tête et pied** : une seule zone commune à tout le site (outils « En-tête » et « Pied »), modifiable
  depuis n'importe quelle page ; les gabarits disparaissent de l'interface (section « Gabarits » de Pages et
  vue du gabarit de D-179 retirées).
- **Conditions** : la branche montrée (Si ou Sinon) est indiquée dans le canevas et dans les calques ; les
  deux branches se modifient ; tout est réglable dans Styles.
- **Menu d'état** : en attente de la réponse de l'utilisateur pour le mode « Classes ».
- À traiter aussi : CI rouge de `abeb1e4` (capture de l'accueil du site cobaye, trois essais, styles
  identiques) ; traductions de `prop-entry-list-help`.

**D-183 — 17/11/2026 — prise (demande de l'utilisateur : « Maquette avant code, rajoute ceci aux règles de Revel »).**
Toute fonction d'interface nouvelle ou refaite est d'abord montrée en maquette et validée par l'utilisateur ; ses
scénarios d'usage deviennent les essais de bout en bout. Pourquoi : D-179 à D-181 ont livré des outils
(gabarits, conditions, cartes reliées au CMS) conformes à mon interprétation et non à sa demande. S'applique
dès le lot 2 (D-182). Règle écrite dans `docs/ETAT.md` (« Reprise » et « Exigences permanentes »).

**D-184 — 17/11/2026 22h — prise (maquettes du lot 2 validées, réponses de l'utilisateur ; complète D-182).**
Maquettes : `tools/maquettes/lot2.mjs` (scènes A à F). Ordre de codage : tout le lot, sans arrêt entre les sujets.
- **Menu d'état** : seulement avec les classes activées (inchangé depuis D-181).
- **Propriété « Couleur »** d'un composant : la valeur est une couleur du site (Paramètres › Couleurs), choisie
  dans leur liste ; aucune couleur libre. Pourquoi : une seule source des couleurs, celle du site.
- **Texte relié au CMS** : plus de saisie sur place au double-clic. Bulle « Champ CMS · collection › champ » avec
  « Remplacer sur cette utilisation », « Modifier ce contenu dans le CMS », « Modifier le composant ». Dans le
  panneau, chaque champ CMS est en lecture seule, avec une icône de modification qui le remplace sur
  l'utilisation (libellé bleu, retour à la valeur du contenu). Dans une liste, le remplacement n'est pas proposé
  (la liste fournit le contenu de chaque carte).
- **En-tête et pied** : interrupteur « Montré sur cette page » ; pas d'en-têtes multiples. Proposé à
  l'utilisateur, en attente : « Variante sur cette page » (la variante du composant de l'en-tête choisie par
  page, par exemple transparente sur une page à grande image), au lieu d'un second en-tête.
- **Liste de collection** : « Contenus : Tous / Choisis » et « Contenus mis en avant » de la 0.7.9 sont réunis
  dans « Automatique / Choix manuel ».
- En suspens : réponse sur « Variante sur cette page ».

**D-185 — 18/11/2026 — prise (lot 2, sujet A : propriété « Couleur » d'un composant ; demandes du 18/11).**
- **Propriété « Couleur »** (`PropKind::Color`) : sa valeur est le nom d'une couleur du site (variable sans
  `--`), choisie dans un menu à pastille, par défaut dans le composant et par utilisation ; jamais de couleur
  libre (D-184). À la création, l'élément racine du composant reçoit `style="--<propriété>: var(--{{
  <propriété> }})"` (commande groupée avec l'ajout) ; dans le composant ouvert, le sélecteur de couleur propose
  la propriété en tête des couleurs du site et écrit `var(--<propriété>)` dans la classe. Pourquoi : la couleur
  d'une utilisation doit atteindre des règles de classe communes à toutes les utilisations ; une variable sur
  la racine le fait sans code visible et sans règle par utilisation (même principe que les variables de
  composant de Framer). Le relevé admet cette valeur de `style` (`editable.rs`).
- **Variantes** : le panneau d'une utilisation ne propose déjà que les variantes créées (constaté dans le code
  de la 0.7.9) ; rien à changer là. Le survol et l'appui restent proposés dans le composant ouvert.
- **Version** : la version qui suit le lot 2 est la **0.8.0**, pas la 0.7.10 (demande du 18/11).
- **Plan du site et redirections** (demande du 18/11, pour la 0.8.0) : le plan lisait déjà les redirections
  (D-181), mais montrait chaque ancienne adresse comme un nœud : le site Alunites Groupe en a 61, le plan
  s'ouvrait à 17 %. Fait : groupe « Redirections » dans la barre des groupes (à côté de « Médias et réseaux »,
  avec leur nombre), masqué à l'ouverture comme les autres ; présent seulement si le site a des redirections.
  Pourquoi un groupe et pas un changement d'adresses : même geste que les groupes existants, et les adresses
  du site restent celles choisies. Non fait, à décider : reprendre dans le site les anciennes adresses
  connues de Google (autre voie donnée par l'utilisateur).
- **Couleurs du site, nombre d'utilisations** : Paramètres › Couleurs ne comptait que les styles ; `--alunites`,
  lue seulement par un attribut, a été supprimée sur le site Alunites Groupe (lueur de la carte devenue grise).
  Fait : les attributs des pages, gabarits et composants et les propriétés « Couleur » sont comptés. Reste : une
  couleur dont le nom vient d'un champ (`var(--{{ marque.data.slug }})`) n'est pas comptée.
- **Dates des flux** (réponse de l'utilisateur, 18/11 10h23) : une date corrigée à la main dans le CMS est
  gardée malgré les reprises suivantes. Le mécanisme existait depuis D-181 (`feed_date`) ; ajouté : un contenu
  repris par la 0.7.9, sans `feed_date`, garde aussi sa date dès la première reprise si elle diffère de celle du
  flux (`feed.rs`). Contrepartie acceptée : pour ces contenus, une correction faite ensuite à la source ne
  remonte plus tant que la date du CMS n'est pas remise à celle du flux. Rappel : Webflow publie « Published
  On », qui change à chaque republication.
- En suspens : « Variante sur cette page » de l'en-tête (D-184) ; montrer dans la fiche du CMS qu'une date est
  corrigée à la main, avec retour à la date du flux (maquette d'abord).

**D-186 — 18/11/2026 13h35 — prise (réponses de l'utilisateur ; complète D-184 et D-185).**
- **En-tête et pied** : « Variante sur cette page » retirée (pas indispensable, « au plus clean »). Il reste
  l'interrupteur « Montré sur cette page ».
- **Anciennes adresses connues de Google** (site Alunites Groupe) : le site reprend ses anciennes adresses au
  lieu de les rediriger. Projets à `/project/<identifiant>/`, actualités à `/<identifiant>/` : 45 des 61
  redirections disparaissent ; 16 restent, pour des pages qui n'existent plus (8 projets retirés, 2
  actualités, `/agence/`, 4 expertises, `/feed.xml`). Script prêt et essayé :
  `passation/alunites-anciennes-adresses.py` (dossier Revel du PC), à lancer dans le dossier du site avec
  Revel 0.8.0 ou plus. Non appliqué au site : la 0.7.9 refuse de générer avec ces adresses (voir le point
  suivant), le site serait resté impubliable. Contrepartie à connaître : les adresses actuelles
  (`/projets/<identifiant>/`, `/actualites/<identifiant>/`), en ligne depuis le 16/11, ne répondront plus,
  sauf à ajouter leurs redirections.
- **Contenu sans la valeur que lit l'adresse de sa collection** (contenu repris d'un flux : pas d'identifiant) :
  il n'a pas de page. Jusque-là `/actualites//` s'écrivait sur la page `/actualites/` et `/{{ slug }}/`
  donnait `//`, refusé (« chemin de page invalide »). Corrigé dans `revel-render` (`prepare`), test
  `contenu_sans_identifiant_sans_page`. Avec lui : `{{ fileSlug }}` (nom du fichier du contenu) se lit dans
  l'adresse d'une collection ; il y était vide (`/projet//`, masqué par un essai).
- **Date corrigée à la main d'un contenu de flux** : à montrer dans la fiche du CMS (libellé bleu, retour à
  la date du flux, date du flux rappelée). Maquette envoyée le 18/11, en attente d'accord (D-183).

**D-187 — 18/11/2026 — prise (lot 2, sujet F : branche montrée d'une condition).**
- Calques : une condition qui a un « Sinon » montre deux lignes, « Si » et « Sinon » (avant : le contenu
  du « Si » directement sous la condition, seul « Sinon » nommé). La branche rendue dans le canevas porte
  un œil ; l'autre et son contenu sont estompés. Une condition sans « Sinon » reste telle quelle.
- Canevas : le repère de sélection d'un élément placé dans une branche porte son nom (« Env badge · Si »).
- Source de l'information : le rendu de l'éditeur note pour chaque condition la ou les branches rendues
  (une condition dans une liste peut rendre les deux) ; envoyé au canevas (`#rv-branches`) puis à
  l'interface (`ready`, `refreshed`). Rien n'est deviné côté interface.
- En suspens : retrait de « Code » du panneau de condition (montré dans la maquette), non fait.

**D-188 — 18/11/2026 14h37 — prise (demande de l'utilisateur pour la 0.8 : vue d'ouverture et barre de l'éditeur).**
- Vue d'ouverture : canevas libre éteint par défaut (page ajustée à la zone). Réglages › Éditeur › Options :
  « Ouvrir en canevas libre » (`start_free`), « Ouvrir avec tous les écrans » (`start_side`, qui entraîne le
  canevas libre : l'autre interrupteur est alors coché et verrouillé). L'état de la dernière séance n'est
  plus gardé (préférences locales `free` et `side` abandonnées) : les boutons de la barre valent pour la
  séance. Pourquoi : un état d'ouverture prévisible, réglé à un seul endroit.
- Barre : à gauche page, langue affichée, thème du site (ordre inversé) ; au milieu de la barre (grille à
  trois zones, centrage exact quelle que soit la largeur des côtés) écrans, canevas libre, tous les écrans,
  gestion des écrans ; à droite règle, zoom, volets, aide, annuler, rétablir.
- Codé sans maquette préalable (écart à D-183) : disposition dictée bouton par bouton par l'utilisateur ;
  captures envoyées pour accord.
- Les essais de bout en bout gardent le canevas libre à l'ouverture (simulateur : `start_free` actif sauf
  paramètre `fitted`) ; la page d'essai du mode simple vérifie l'ouverture par défaut.

**D-189 — 18/11/2026 17h00 — prise (fin du lot 2 : sujets C, D, E ; maquette de la date validée ; projet retiré).**
Réponses de l'utilisateur : maquette « date corrigée » validée ; projet « Animation du réseau après-vente »
(Alunites) retiré du site ; finir le lot 2 ; adresses du site sans redirection.
- **C — outil « Champ CMS »** : intégré au réglage existant « Relié au contenu » (pas de second mécanisme).
  Panneau : Collection (collections lisibles à cet endroit : liste, contenu du composant, modèle de pages,
  listes englobantes), Champ, « Affiché en » texte / image / lien (élément sans autre élément dedans, converti
  en gardant identifiant et classe), « Si le champ est vide » : garder, masquer (condition sur le champ),
  selon une condition (éditeur complet). Outil dans Ajouter › CMS : texte relié d'emblée au premier champ de
  texte (titre de préférence) ; hors d'un endroit relié, texte écrit. Calques : « Champ CMS · Collection ›
  Champ » ; une valeur lue seule est nommée par son champ, plus d'accolades. Écart à la maquette : le menu
  Collection ne propose que les collections lisibles à l'endroit, pas toutes celles du site (une autre ne
  donnerait aucune valeur).
- **D — liste de collection** : « Contenus » en deux boutons, Automatique / Choix manuel (ancien menu « Tous /
  Choisis ») ; en choix manuel les réglages automatiques (mis en premier, filtres, tri, nombre, début) sont
  masqués. Une liste lisible par ses menus ne montre plus son expression (« Liste parcourue », « Nom de chaque
  élément »). Listes écrites à la main : `| limit(n)` en dernier filtre est lu comme le nombre maximal (liste
  « Projets » de l'accueil du cobaye) ; à la première modification, l'expression est réécrite sous la forme
  de Revel (`(…)[:n]`, même liste). Les autres formes restent « liste calculée ».
- **E — en-tête et pied** : zone = élément `header` / `footer` d'un gabarit, ou utilisation d'un composant
  dont la racine l'est. Calques de toute page : « En-tête » / « Pied » avec la pastille « Tout le site » ;
  repère du canevas « En-tête · tout le site ». Panneau : « Montré sur cette page » — éteint, la page reçoit
  `sans_entete` / `sans_pied` ; la première fois, le gabarit reçoit l'interrupteur (propriété de gabarit
  existante, D-96, visible aussi dans les réglages de la page) et la zone la condition `not sans_entete`.
  Pas de variante par page (D-186). Outils « En-tête » et « Pied » (Ajouter › CMS) : zone existante
  sélectionnée, sinon posée au début ou à la fin du `body` du gabarit. Section « Gabarits » du panneau Pages
  retirée ; la vue d'un gabarit seul n'a plus d'entrée dans l'interface.
- **Date corrigée d'un contenu de flux** : dans la fiche, libellé marqué, bouton de retour à la date du flux,
  « Flux : <date> » sous le champ ; `feed_date` n'est pas montré comme champ.
- **Site Alunites Groupe** : projet retiré (contenus FR et EN, médias, suggestions de 7 projets) ; son ancienne
  adresse `/project/animation-reseau-apres-vente/` renvoie vers `/projets/`.
- **Reportés après l'installation de la 0.8.0** (le site s'ouvre aujourd'hui avec la 0.7.9, qui ne lit ni la
  propriété « Couleur » ni les nouvelles adresses) : cartes de marque en propriétés et retrait de la collection
  « Marques » ; script des anciennes adresses.

**D-190 — 19/11/2026 — prise (retours sur le site Alunites Groupe ; corrections de Revel pour la 0.8.1).**
- **Saut du défilement sur Mac** (vidéo de l'utilisateur, Chrome) : le défilement doux de Revel croyait
  reconnaître le pavé tactile à ses pas décimaux ; Chrome sur macOS envoie des pas entiers pour le pavé
  tactile (source : Bugzilla 1235192, journaux comparés), donc le script ajoutait sa propre inertie à celle
  du système, cible accumulée puis rattrapée d'un coup. Correction : sur les appareils Apple (Mac, iPhone,
  iPad), défilement du navigateur, sans script. Les animations d'apparition (opacité, transformations) et le
  bouton « Afficher plus » (liste manuelle) ne changent pas la hauteur de la page : hors de cause.
- **Diaporama** : réglages `fade[=<s>]` (fondu enchaîné, diapositives superposées) et `hold=off` (lecture non
  suspendue sous le pointeur, pour un fond de section). Réglages gardés par le panneau du carrousel (options
  inconnues conservées) ; interrupteurs dans le panneau : à maquetter.
- **Filtres de liste** : un bouton dont la valeur ne correspond à aucun élément de la page est masqué
  (Cadence et Superficiel dans les actualités).
- **Site** (script `passation/alunites-modifs-1911.py`, appliqué sur le PC, idempotent) : Hero d'accueil à
  4 images en fondu (6 s), points, fondu vers la couleur de fond du thème sous le texte ; section « Cultiver
  les différences » masquée (condition `false`, contenu gardé) ; page « Le Groupe » en brouillon, adresses
  redirigées vers l'accueil, liens retirés (navigation, pied) ; « Alunites Event » (« Event » 2 pt plus petit)
  dans la navigation et les filtres ; case « evenementiel » sur 7 projets et 4 articles (choix de
  l'utilisateur) ; filtre « Alunites Event » et liste de la page Alunites Event limités à ces contenus ;
  titres des cartes de projets et d'articles −2 pt, graisse 700 → 600 (carte « à la une » inchangée).
- **Spams du formulaire** : en suspens (choix de l'utilisateur entre les protections proposées).

**D-191 — 22/11/2026 — prise (accord de l'utilisateur : champs CMS dans un composant, anti-spam ; pour la 0.8.1).**
- **Champs CMS dans un composant** (maquettes avant / après validées le 22/11). Le composant ne connaît plus
  de collection : il déclare des propriétés typées (texte, image, lien, oui / non, liste), avec valeur par
  défaut. Chaque utilisation dit d'où viennent ses valeurs, dans une seule section « Contenu » :
  - Source : la liste de collection qui l'entoure (pastille « Liste · Projets », « Régler » sélectionne la
    liste) ; sinon menus « Collection » et « Contenu » ; sinon « Aucune » (valeurs écrites).
  - Une ligne par propriété : menu du champ lu (ou « Page du contenu », ou « Écrire… ») et, à côté, la valeur
    lue par l'utilisation sélectionnée. Collection choisie : chaque propriété libre prend le champ de même
    nom ; les autres sont signalées (icône de liaison colorée).
  - « Modifier ce contenu dans le CMS » ouvre le contenu montré.
  - Forme écrite : `item.data.titre` dans une liste ; `(collections.c | by_slug("x")).data.titre` pour un
    contenu choisi ; `((collections.c)[0]).data.titre` pour le premier. Collection et contenu se relisent
    dans ces valeurs (rien d'autre n'est enregistré) ; une collection choisie sans aucune propriété reliée
    n'est gardée que le temps de la séance.
- **Conversion** des composants à propriété « Contenu » (D-174) : à l'ouverture du site dans l'éditeur, une
  étape annulable, message d'information. Une propriété par champ lu (`projet.data.title` → `title`), une
  pour l'adresse de la page (`lien_page`) ; nature d'après le CMS, sinon la valeur (liste, oui / non), sinon
  l'usage (source d'image, adresse de lien) ; valeur par défaut : celle du premier contenu. Chaque
  utilisation reçoit ses valeurs. Un composant qui lit son contenu autrement que par ses champs n'est pas
  converti. Site généré identique avant et après (vérifié sur le site Alunites, 168 fichiers), hors le reste
  d'import retiré (`data-revel-filter=""` sur une carte filtrée : l'ancien « Filtres de liste / Valeur »).
  Le type « Contenu » n'est plus proposé à la création ; le rendu le lit toujours.
- **Écarts aux maquettes**, signalés à l'utilisateur :
  - pas de propriété « Filtres » en pastilles : les étiquettes de filtre suivent les propriétés que le
    composant lit déjà (sur Alunites : Source et Événementiel) ;
  - le lien d'une carte Alunites tient en deux propriétés (lien externe, page), le composant lisant les deux ;
  - dans une liste, « Écrire… » vaut pour toutes les cartes (une seule utilisation) ; par carte seulement
    hors d'une liste ;
  - libellés des propriétés : ceux des champs du CMS (noms des champs quand le schéma n'en donne pas).
- **Anti-spam des formulaires** : preuve d'un navigateur — durée écoulée depuis l'affichage (3 s au moins)
  et valeur calculée par le script de la page avec une clé du site (FNV-1a, même calcul en PHP), vérifiées
  par `revel-form.php` ; champ piège renommé `website_url` (ancien nom encore reconnu). Sans JavaScript, le
  formulaire ne part plus (message). Essayé avec PHP : envoi direct refusé, envoi en moins de 3 s refusé,
  envoi normal accepté. Pas de service tiers. En suspens : CAPTCHA si des envois à la main persistent.

**D-192 — 23/11/2026 — prise (choix de l'utilisateur : « toutes mes apps en open source et signées »).**
**Licence MIT et signature Windows par SignPath Foundation.** Clôt D-08.
- **Licence** : MIT, titulaire « MATV Studio » (champ `authors` des manifestes), fichier `LICENSE` à la racine,
  `license = "MIT"` dans tous les manifestes Cargo et dans le paquet Tauri. Elle vaut à partir de ce commit ;
  les versions publiées avant (jusqu'à 0.8.0, et le brouillon 0.8.1 construit sur `28b4a85`) n'en portent pas.
  Une licence ouverte ne se retire pas des versions qui la portent.
- **Pourquoi MIT** : la plus simple des licences approuvées par l'OSI, compatible avec toutes les dépendances
  (relevé : `docs/LICENCES-TIERCES.md` — 812 et 206 paquets Rust, tous sous licence OSI, aucun copyleft fort
  imposé ; MPL-2.0 pour cinq paquets utilisés sans modification), et condition d'entrée du programme de
  SignPath Foundation (licence OSI, pas de double licence commerciale, aucun composant propriétaire).
- **Signature Windows** : SignPath Foundation (gratuit pour les projets libres ; éditeur affiché « SignPath
  Foundation »). Conditions vérifiées sur signpath.org/terms le 23/11/2026 : projet publié et maintenu,
  constructions par la CI depuis le dépôt public, double authentification de l'équipe, rôles déclarés,
  page « Code signing policy » liée depuis l'accueil du dépôt, désinstallation, approbation manuelle de
  chaque version, métadonnées produit identiques d'une version à l'autre. Page écrite :
  `docs/CODE-SIGNING-POLICY.md` (en anglais, lue par SignPath), liée depuis le README.
- **Confidentialité déclarée** : aucune télémétrie ; seule connexion non demandée, la vérification des mises
  à jour au lancement (liste des versions sur GitHub, rien d'envoyé). À revoir si une fonction en ajoute.
- **Reste à l'utilisateur** (réglages et secrets, hors de portée de Claude) : double authentification sur
  GitHub, candidature sur signpath.org, installation de l'application GitHub de SignPath, déclaration des
  identifiants et du jeton dans le dépôt.
- **En suspens** : l'étape SignPath du workflow `release.yml` (elle demande l'identifiant d'organisation, le
  projet et la politique de signature donnés par SignPath à l'acceptation ; elle s'insère entre la
  construction Windows et le calcul des signatures de mise à jour, qui doivent porter sur les fichiers
  signés) ; la signature macOS (compte Apple Developer payant, D-09) ; la licence des autres apps internes.
- Titulaire du droit d'auteur « MATV Studio » : confirmé par l'utilisateur le 23/11/2026.
- Historique contrôlé le 23/11/2026 (233 commits) : aucune clé ni mot de passe (détail : `docs/AUDIT-1.0.md`, § 12).

**D-193 — 23/11/2026 — prise (réponses de l'utilisateur au § 10 de l'audit, `docs/AUDIT-1.0.md`).**
**Cap de la 1.0 : styles par calque, vocabulaire de Framer, volet d'insertion resserré.**
- **Styles propres à chaque calque**, comme Framer (choix de l'utilisateur) : régler un élément ne change que
  lui ; partage explicite par styles de texte nommés, couleurs du site et composants ; survol et appui libres
  sur un calque ; noms de calques d'après le rôle ; cibles des Actions par calque. Le modèle par classes
  devient le mode avancé (sites importés). Pourquoi : en mode sans classes, les styles restaient partagés par
  classe sans que l'utilisateur le voie (un sous-titre réglé en changeait quatre ; survol d'un composant
  débordant sur ses voisins).
- **Composant créé depuis une carte de liste** (proposition de Claude, demandée « logique pour un utilisateur
  ordinaire, avec du contrôle ») : une propriété par champ lu, créée d'office ; source « Liste » gardée dans
  la liste ; Collection et Contenu à choisir ailleurs ; chaque propriété reste réglable ; une étape annulable.
  Étend la conversion de D-191 à la création.
- **Volet d'insertion** (confié à Claude) : premier niveau resserré, niches sous « Plus », « Colonnes »
  retiré, champs de formulaire, liste déroulante et pastille de panier ajoutés ; tableau au § 10.3.
- **Première publication** (choix de l'utilisateur) : état actuel gardé ; champ « Message de l'hébergeur »
  supprimé ; bouton « Assistant » en trois étapes vers GitHub Pages ; info-bulle : hébergement payant possible,
  pas nécessaire.
- **Vocabulaire** (choix de l'utilisateur) : celui de Framer, traduit ; table au § 10.5 (Insérer, Pile, Cadre,
  Texte, Accroche, Diaporama, Ticker, Variable, Effets, Style de texte, Bouton de thème…).
- **Ordre** : pannes bloquantes, styles par calque, logique des outils, libellés.
- Les coordonnées d'Alunites citées en exemple dans le dépôt sont gardées (voulu par l'utilisateur).
- Toute modification d'interface issue de ces décisions passe par une maquette validée (D-183).
- En suspens : passage des sites existants au modèle par calque ; réglages « écrits en code » des blocs ; mots
  de la table à confirmer écran par écran ; signature macOS.
- Compléments du 23/11/2026, 17 h 59 (utilisateur) : un site à classes doit pouvoir être **converti** au modèle
  par calque (règles reportées sur les calques, classes de texte partagées devenues styles de texte, site
  généré identique, étape annulable) ; pour les templates, trois voies notées au § 10.1 du rapport (source
  unique convertie, deux versions, ou refonte dans le seul modèle par calque), choix après les premiers essais
  de conversion. Ordre des travaux validé. Réglages « écrits en code » d'Alunites : après la discussion de
  l'audit.

**D-194 — 23/11/2026 — prise (demande de l'utilisateur).** **Templates dans un dossier du dépôt Revel, lus en
ligne, avant la prochaine version.** But : ajouter ou retirer un template sans publier une version. État : 27
templates compilés dans l'application (`crates/revel-core/templates`, 1,4 Mo sur environ 50 Mo : gain de poids
faible) ; lecteur de catalogue en ligne déjà présent (`catalog.json` d'un dépôt GitHub, `sharing.rs`) ;
catalogue par défaut `matv-studio/revel-templates` introuvable. À faire : dossier `templates/` et son
`catalog.json` dans le dépôt Revel, lecteur acceptant un dossier d'un dépôt, catalogue par défaut sur ce
dossier, seul « Vide » encore compilé, liste gardée en cache. En suspens : « Nouveau site » sans connexion ;
« Vitrine » embarqué ou non (maquette).
- **Fait le 23/11/2026 (cœur, sans changement d'écran)** : dossier `templates/` à la racine du dépôt (les 27
  modèles, `_kit/`, `photos.json`, `index.json`, `LISEZMOI.md`), `build.rs` supprimé : plus aucun modèle n'est
  compilé (« Vide » est écrit par le code). Le format écrit des modèles est gardé (deux langues
  `[[fr||en]]`, couleurs claires ou sombres au choix à la création) : c'est pourquoi le dossier est lu tel
  quel plutôt que transformé en « paquets » de catalogue, qui n'ont qu'une langue. `index.json` liste les
  fichiers et porte une révision (empreinte des chemins et des contenus) ; `revel templates-index templates`
  le refait, un essai le contrôle. `Core::templates_sync` lit l'index en ligne
  (`raw.githubusercontent.com/matv-studio/Revel/HEAD/templates/`), télécharge les fichiers par huit, vérifie
  que le jeu reçu correspond à la révision, remplace la copie gardée dans le dossier de réglages
  (`templates/`) d'un seul geste ; même révision : rien n'est téléchargé ; hors ligne ou jeu incomplet : la
  copie gardée reste. Lecture lancée en arrière-plan au démarrage de l'application, et par `revel templates` /
  `revel new`. Développement et essais : `REVEL_TEMPLATES_DIR`, sinon le dossier `templates/` des sources à
  côté du programme. Catalogues ajoutés par l'utilisateur : mécanisme existant (Réglages › Templates, « + » de
  « Nouveau site »), inchangé ; le catalogue par défaut `matv-studio/revel-templates`, introuvable, est retiré.
  Reste : les écrans (onglet des modèles de Revel, état hors connexion) après validation de la maquette ;
  conséquence à connaître : une version de Revel plus ancienne lit le même dossier `HEAD`, un modèle qui
  demande une nouveauté du cœur doit donc attendre que la version qui la porte soit répandue (piste : champ
  « version minimale » par modèle).
- Compléments du 23/11/2026, 19 h 56 (utilisateur) : conversion inverse (calques → classes) par regroupement,
  par type d'outil, des valeurs identiques ou proches ; **une seule source par template**, convertie à la
  création selon le mode (chaque template sert d'essai de la conversion) ; catalogue de Revel lu par défaut,
  avec la possibilité d'ajouter un dépôt ; ordre des travaux lancé.

**D-195 — 24/11/2026 — prise (maquettes validées par l'utilisateur, retours du 24/11).**
**Composants : variables tirées d'une carte de liste, liste des variables sur la racine seulement ; Nouveau
site : état des modèles ; molette sur les panneaux.**
- **Rappel de principe** (utilisateur) : un composant est une structure réutilisée partout sur le site,
  modifiée en un seul endroit ; le CMS n'est qu'une source possible pour les valeurs de ses variables. Rien
  dans les composants n'est propre au CMS.
- **Composant créé depuis une carte de liste** (audit P2, D-193) : `MakeComponent` puis
  `entryprops::lift_list_reads` dans une seule étape ; chaque lecture du contenu (`item.data.x`, `item.url`)
  devient une variable nommée d'après le champ, les utilisations dans la liste reçoivent `item.data.x` ; rendu
  inchangé ; message « Composant créé · n variables reliées aux champs » avec « Annuler ». `MakeComponent`
  accepte `use_id` (identifiant de l'utilisation connu d'avance, pour enchaîner dans la même étape). Sur un
  élément sans CMS, rien ne change.
- **Liste des variables** : montrée seulement quand la racine du composant est sélectionnée ; sur un
  sous-élément, seule sa propre liaison (« tout afficher partout perd l'utilisateur »). Comme Framer et Figma.
- Mot « Variables » (français, anglais) à la place de « Propriétés » ; les autres langues suivront avec la
  passe de vocabulaire (D-193, § 10.5).
- **Nouveau site** : onglet « Revel », onglet « Catalogue » seulement si un dépôt est ajouté, nuage d'état
  (à jour · n, copie gardée le…, hors connexion la première fois) avec « Réessayer » ; « Vitrine » n'est pas
  embarqué. Sélecteur de fond : texte qui ne dépasse plus.
- **Panneaux** : molette sur le haut d'un groupe = onglet suivant ou précédent (comme After Effects) ;
  Ctrl + molette dans Médias = taille des vignettes.
- Captures et essais d'interface en anglais, langue par défaut de Revel (demande de l'utilisateur).
- En suspens, maquettes envoyées le 24/11 : outil « CMS field » (calque et icône propres, choix de la
  collection et du champ dans le panneau, état sans collection) ; menus du clic droit dans le vide de Pages,
  Components, Assets. Hors d'une liste, source par défaut d'une utilisation : valeurs écrites (le choix
  Collection / Contenu reste proposé) — à revoir avec la maquette du champ CMS. Essais de bout en bout à
  écrire pour : composant depuis une carte de liste, molette, état des modèles (vérifiés à la main le 24/11).

**D-196 — 24/11/2026 — prise (maquettes validées par l'utilisateur le 24/11, avec ajustements).**
**Composants : bouton de liaison à côté du champ, variables nommées d'après le calque, types nombre, choix et
icône ; outil « CMS field » hors d'une liste ; menus du clic droit des panneaux.**
- **Bouton de liaison** (comme le « + » de Framer) : posé à côté du champ de l'image (source), du lien
  (destination) et de l'icône ; relié, il porte le nom de la variable et son menu délie. Texte et visibilité,
  qui n'ont pas de champ dans le panneau, gardent une ligne avec le même bouton. La rubrique à part « Relié aux
  variables » disparaît.
- **Nom d'une nouvelle variable** : celui du calque relié (« Paragraph » → `paragraph`), numéroté s'il est
  pris ; créée par « + » sur la racine, elle garde le nom de son type.
- **Types ajoutés** (`PropKind`) : `number` (nombre écrit sans guillemets), `choice` (valeurs dans
  `PropDef::options`, la première par défaut), `icon` (nom d'une icône de Revel ; l'élément icône relié porte
  `data-icon="{{ nom }}"` et dessine `{{ nom | icon }}`, filtre ajouté au rendu). Un texte se relie à une
  variable texte, nombre ou choix.
- **Rubrique « Content » d'une utilisation** (retour du 24/11) : montrée seulement quand l'utilisation lit un
  champ du CMS ; un composant sans champ du CMS ne montre que ses variables. Une fois montrée, elle reste le
  temps de la session (« None » choisi par erreur).
- **Outil « CMS field »** : l'élément posé porte `data-revel-base="cms-field"` (nom « CMS field » et icône du
  CMS dans les calques, le canevas et la barre d'état). Hors d'une liste : sans collection, « No collection
  yet » et « Create a collection » (ouvre le CMS) ; sinon menu « Collections » à coches, « All » en tête
  (seulement s'il y a plusieurs collections), puis menu « Field » — champs communs d'abord, trait, puis champs
  d'une seule collection, chacun avec le nom de sa collection (pas de « only », tous sélectionnables) ; une
  seule collection : la valeur du premier contenu à droite. Champ choisi : le calque prend le nom du champ,
  suivi de la valeur lue ; réglages Collection, Item (premier contenu ou un contenu précis), Shown as, If
  empty. Expressions écrites sans filtre ni parenthèse pour être lues par les réglages existants :
  `collections.<id>[0].data.<champ>` et `entry("<id>", "<slug>").data.<champ>` (fonction `entry` ajoutée au
  rendu). Dans une liste ou un modèle de pages, l'outil garde ses menus d'avant.
- **Menus du clic droit** (tout le panneau hors lignes, pied compris : un panneau plein n'a plus de vide) :
  Pages (New page, New folder, New CMS page…, Paste page, Collapse all folders, Open site map) ; Components
  (New component, New folder, Create from selection, Paste component, Collapse all folders) ; Assets (Import
  files…, New folder, Paste from clipboard, Larger / Smaller thumbnails, Show unused only, Show in file
  explorer). « Copy » ajouté aux menus d'une page et d'un composant. `make::duplicate_component`,
  `make::empty_component`. Composant ouvert dont la création est annulée : retour à la page.
- **Essais** : `tools/panels-e2e.mjs` (23 essais, interface en anglais, site du modèle Portfolio + seconde
  collection), étape de CI « Panneaux et composants » ; essais de rendu `variables_nombre_choix_icone`,
  `champ_cms_hors_liste` ; essai `composant_copie_et_composant_vide`. Couvre aussi le lot D-195 (molette,
  Ctrl+molette, carte de liste, variables sur la racine, état des modèles).
- **En suspens** : « Import pages… » et « Import component… » de la maquette ne sont pas faits (source à
  définir : fichier HTML, autre site Revel, fichier de composant) ; variable nombre reliée à une valeur de
  style (opacité, rayon) ; icône importée (fichier SVG) comme valeur d'une variable icône ; hors d'une liste,
  relier une variable d'une utilisation à un champ du CMS n'a plus d'entrée depuis que « Content » est masquée ;
  barre du panneau Styles : nom « CMS field » gardé une fois le champ choisi ; « Paste from clipboard » et
  « Show in file explorer » non essayés dans l'application de bureau ; mot « Variables » dans 13 langues.

**D-197 — 24/11/2026 — prise (retours de l'utilisateur du 24/11, 10 h 18).**
**Outil « CMS field » sans texte libre ; Collection list alignée ; langue d'un nouveau site ; bibliothèques
d'icônes et imports en maquette.**
- **« CMS field »** : le champ « Text » du panneau n'est plus montré pour cet outil (tout passe par le champ
  choisi). Hors d'une liste, relier une variable d'une utilisation de composant au CMS n'a pas d'entrée propre :
  on passe par l'outil « CMS field » (choix de l'utilisateur).
- **Collection list** : même menu « Collections » à coches que « CMS field » (« All » en tête, la première
  cochée donne ses champs au filtre et au tri, au moins une cochée), à la place du menu « Collection », des
  pastilles « + collection » et du menu « + Another collection » ; même état sans collection (« No collection
  yet », « Create a collection ») ; même tête avec l'icône du CMS.
- **Nouveau site** : langue proposée = langue de l'application en cours (réglage « système » compris).
- **Bibliothèques d'icônes** (maquette envoyée, à valider) : réglage « Icon libraries » sur le modèle des
  catalogues de templates (dépôts GitHub ajoutés ou retirés, dossier des SVG, lus en ligne une fois puis
  gardés), menu « Library » dans le panneau Styles avec « All ». Proposées par défaut, licences vérifiées :
  Lucide (ISC), Tabler (MIT), Phosphor (MIT), Heroicons (MIT), Feather (MIT). Remix Icon écartée par défaut :
  statut de licence signalé comme incertain (Fedora). Une icône choisie est copiée dans le site.
- **Imports** (maquette envoyée, à valider) : « Import pages… » depuis un autre site Revel ou des fichiers
  HTML ; « Import component… » depuis un autre site Revel, un fichier de composant ou un fichier HTML ;
  « Export component… » dans le menu d'un composant.
- **Icône des réglages** (retour du 24/11, 10 h 38) : tout ce qui ouvre des réglages porte l'icône de rouage
  de l'application (`icons::settings`), jamais un autre dessin ; « Manage libraries… » compris.

**D-198 — 24/11/2026 — prise.**
**Ordre de la suite (utilisateur, 24/11 à 14 h) et pannes mineures de l'audit.**
- **Ordre** : pannes mineures → logique des outils (§ 4 de l'audit) → bibliothèques d'icônes → imports →
  styles par calque → vocabulaire → tâche finale : refaire les templates, le site Alunites et le site MATV, en
  essayant pendant la refonte des templates la conversion « classes > sans classes » et « sans classes >
  classes ». Les outils passent avant les styles par calque pour ne pas refaire deux fois les templates.
- **Bibliothèques d'icônes** (maquette validée) : la liste des noms est téléchargée, chaque dessin à
  l'affichage ; Revel et Lucide actives au départ, les autres ajoutées en un clic.
- **Imports** (maquette validée) : pages depuis un site Revel ou des fichiers HTML ; composant depuis un site
  Revel, un fichier de composant ou un fichier HTML ; « Export component… ».
- **Pannes corrigées** : P6, P7, P8, P9, P10, P12, P13, P15 (détail dans `docs/AUDIT-1.0.md`, § 2).
- **En suspens** : P5 — afficher le libellé d'un champ « Choix » demande que le rendu connaisse le schéma des
  collections (`.pages.yml`), aujourd'hui lu seulement par l'éditeur ; à décider : libellé partout, ou réglage
  « libellé / valeur » sur l'élément. P11 et P14 : à reproduire (poignée de largeur d'une image de bloc ;
  taille des textes d'un import Framer).
- **Logique des outils, maquettes validées le 24/11 à 15 h 44** : liens de la barre de navigation (liste à
  poignée, nom modifiable, cible, « Add a link » qui propose les pages, une adresse web ou une nouvelle page) ;
  champs d'un formulaire (liste, « Add a field » : texte, e-mail, téléphone, texte long, liste déroulante, case,
  boutons radio, date, fichier ; après l'envoi : message ou page ; destinataire propre au formulaire) ; filtres
  de liste (liste et champ choisis, boutons tirés des valeurs du champ, bouton « All », plusieurs valeurs) ;
  assistant de première publication en trois étapes vers GitHub Pages, avec l'info-bulle sur l'hébergement
  payant. **Refusé** : le bouton « Insert » en tête de barre (doublon de l'onglet d'ajout) ; l'onglet « Add »
  sera renommé « Insert » lors de la passe de vocabulaire.

**D-199 — 24/11/2026 — prise (maquettes validées le 24/11 à 15 h 44).**
**Logique des outils, premier envoi : liens de la barre de navigation, champs de formulaire, pose d'une
section.**
- **Liens d'une barre de navigation** (audit M6) : chaque ligne montre la destination ; « Add a link » propose
  les pages du site, « Web address… » et « New page… » (la page est créée et le lien posé) ; le lien ajouté
  reprend la forme du dernier (`make::add_link_item`) avec son texte et sa destination, plus une copie à
  corriger. Renommage sur place par le crayon de la ligne (`make::rename_link_item`) : le clic sur une ligne
  sélectionne déjà le lien dans la page, le double-clic n'était donc pas disponible.
- **Champs d'un formulaire** (audit M7, module `revel_edit::formfields`) : liste des champs (nom, sorte,
  astérisque des champs obligatoires, monter, descendre, retirer, renommer), « Add a field » : texte, e-mail,
  téléphone, texte long, liste déroulante, case, boutons radio, date, fichier. Le champ ajouté reprend les
  classes d'un champ déjà là, se place avant le bouton d'envoi, et son nom est demandé aussitôt. Une liste
  déroulante montre ses valeurs sous sa ligne. Le clic sur une ligne garde le formulaire sélectionné.
- **Section complète** (audit M2) : posée après la section qui contient la sélection, pas dedans
  (`insert::section_point`).
- **En suspens, second envoi** : « After sending » (message ou page) et destinataire propres à un formulaire
  (le script d'envoi est commun au site : `revel-render/src/forms.rs` à étendre) ; filtres de liste ; assistant
  de première publication ; M3 (marges des outils hors bloc), M9 (titre du modèle Vide), M10 (corps « Texte »
  dans les champs d'un contenu), M12 (double-clic, Ctrl+B / Ctrl+I), M13 (contenus d'attente). Le nom
  technique d'un champ ajouté (`name`) vient de sa sorte et ne suit pas le renommage.

**D-200 — 25/11/2026 — prise (retours de l'utilisateur du 25/11, 0 h 42).**
**Poignée de glisser à la place des flèches ; libellé des champs « choix » (audit P5).**
- **Poignée** (`apps/revel-ui/src/sortable.rs`) : les listes ordonnées n'ont plus de flèches « monter » et
  « descendre » ; une poignée à gauche de la ligne se glisse (un trait montre la place d'arrivée, côte à côte
  pour une grille de vignettes) ou se pilote au clavier (flèches haut et bas une fois la poignée en focus).
  Listes concernées : liens de la barre de navigation, champs d'un formulaire, parties d'un outil (onglets,
  diapositives…), contenus choisis d'une liste (deux endroits), menus du site (deux endroits), réseaux
  sociaux de l'organisation, liste répétée et valeurs en lignes du CMS, valeurs d'un champ choix du schéma.
  Restent à flèches, car elles déplacent l'élément sélectionné et non une ligne : « monter / descendre » des
  calques et l'en-tête d'un champ du schéma. Lignes des liens et des champs dessinées comme la maquette :
  encadrées, nom en gras, cible à droite en chasse fixe, corbeille visible, autres actions au survol.
- **P5** (décision laissée à Claude : le plus logique et le moins encombrant) : le libellé d'un champ
  « choix » s'affiche partout où un texte de la page montre ce champ tel quel (`item.data.discipline`,
  `entry(…).data.x`, `collections.c[0].data.x`) ; adresses, attributs, filtres, tris et expressions calculées
  gardent la valeur. Pas de réglage. Le projet lit les libellés dans `.pages.yml` au chargement des contenus
  (`Entry::labels`), le rendu les expose (`item.labels.<champ>`). Un texte qui montre un libellé n'est pas
  modifiable dans le canevas (il écrirait le libellé à la place de la valeur). Contrepartie : un site qui
  affichait la valeur affiche désormais le libellé. Choix multiple : pas concerné (liste affichée par un
  filtre).


## D-201 — Formulaire : après l'envoi et destinataire propres à chaque formulaire ; copie du cobaye (25/11/2026)

- **Après l'envoi** (maquette `tools-3-form-fields` validée le 24/11) : sur un formulaire « envoyé par le
  site », bascule « Message / Ouvrir une page », message propre (vide : celui du site), page choisie dans
  la liste des pages, et « Envoyer à » (vide : adresse du site). Attributs du `<form>` :
  `data-revel-success`, `data-revel-redirect`, `data-revel-to`.
- **Pourquoi des attributs lus à la publication** : l'adresse ne doit jamais venir du visiteur ni figurer
  dans la page. Le rendu relève les réglages de chaque formulaire (`forms::FormSpec`), les retire des
  pages publiées (gardés dans le canevas) et les écrit dans le script PHP (`const FORMS`, par nom de
  formulaire). Ordre du destinataire : valeur du champ d'aiguillage (D-170), sinon adresse du
  formulaire, sinon adresse du site. Page ouverte : chemin du site seulement (`/…`) ; réponse JSON
  `redirect` suivie par le script de la page, ou `303` sans script.
- L'avertissement « adresse de réception absente » ne tombe plus si chaque formulaire a la sienne.
- Deux formulaires de même nom : les réglages du premier trouvé s'appliquent aux deux (le nom est la clé).
- **Cobaye** : le dépôt `matv-studio/alunites-revel-lab` a été supprimé de GitHub le 25/11. Copie de
  référence sur le PC, `C:\MATV_LOCAL\_Interne\Revel\cobaye\` (dépôt git complet au commit `0dd004c` et
  fichier `alunites-revel-lab.bundle`). Les essais locaux lisent la copie du conteneur. En suspens : les
  trois tâches de CI qui lisent le dépôt GitHub restent rouges tant qu'il n'est pas restauré ou recréé.

## D-202 — Filtres de liste reliés à un champ, archivage des contenus, ajout d'hébergement, réglages centrés, « Versions » (25/11/2026)

- **Filtres de liste** (audit M5, maquette `tools-4-list-filters` validée le 24/11) : l'outil « List
  filters » se règle par la **liste** de la page (choisie d'office quand il n'y en a qu'une) et le **champ**
  (choix, texte court, liste de textes). Les boutons ne sont plus écrits un à un : une boucle
  `collections.<c> | filter_values("<champ>")` donne une clé (`slugify`) et le libellé du choix ; chaque
  contenu de la liste porte `data-revel-tags="{{ item | filter_tags('<champ>') }}"` ; l'élément qui contient
  la liste porte `data-revel-filter-scope="<paramètre>"`, et le script ne filtre que dans celle-ci (deux
  listes sur une page ne se gênent plus). Bouton « All » facultatif et renommable ; « Several values at
  once » (`data-revel-filter-multi`, valeurs séparées par des virgules dans l'adresse ; un contenu est
  montré s'il a l'une d'elles). Boutons d'avant (écrits à la main) gardés tant qu'aucun champ n'est choisi.
  Module `revel_edit::listfilters`, fonctions communes `revel_model::filter_values` / `field_strings`.
- Contenu de la liste dessiné par un **composant** : étiquettes posées sur l'utilisation ; les attributs
  d'une utilisation écrits avec une expression sont désormais évalués là où elle est (contexte de la
  liste) avant d'être posés sur la racine du composant.
- Options d'un champ « choix » : clé `value` (Pages CMS) **ou** `name` (choix écrits par Revel) acceptée
  pour les libellés (P5 ne lisait que `value`).
- **Archiver / désarchiver** un contenu (demande du 25/11, maquettes validées) : `published: false`, que lit
  déjà le site Alunites sous Eleventy et que le rendu écartait déjà (pages, listes, plan du site) ; comme
  l'archivage de Webflow (retiré du site à la publication suivante, gardé dans le CMS). « Archive » au clic
  droit d'un contenu et en icône au pied de sa fiche ; « Unarchive » au pied de la fiche et au clic droit ;
  puce « Archived », ligne estompée. Option **a** retenue faute de réponse : `draft: true` (imports)
  s'affiche aussi « Archived » et « Unarchive » retire les deux marqueurs (un seul état à comprendre).
- **Ajout d'hébergement** (demande du 25/11, maquettes validées) : « Go live » devient « Add hosting »
  (libellé déjà employé par le bouton d'ajout) ; plus de dossier `www/` d'office : après la connexion, liste
  des dossiers trouvés sur le serveur, rien de coché ; le dossier proposé est signalé (« Matches
  <domaine> » ou « Host's usual folder ») ; « Other folder… » pour taper un chemin ; « Save » et « Publish »
  grisés tant qu'aucun dossier n'est choisi. Champ « Message from your host » retiré (maquette de
  l'assistant validée le 24/11) ; un message collé dans un champ reste reconnu. `HostProbe.domain` ajouté.
- **Réglages centrés** (demande du 25/11) : bloc de réglages centré dans la zone à toutes les largeurs ;
  titre de partie dans la marge gauche s'il y tient, au-dessus sinon (requêtes de conteneur, plus de seuil
  sur la fenêtre). Barre de statistiques du site (pages, gabarits, composants, collections, environnement)
  retirée des réglages du site.
- **« Backup » devient « Versions »** dans les 15 langues, « Back up » devient « Save version ». Restaurer
  une version existait (bouton en haut du détail d'une version ancienne) mais restait introuvable : flèche
  au survol de chaque version sauf la dernière (icône seule, « Restore » en info-bulle, à la demande de
  l'utilisateur) et « Restore » / « Copy version ID » au clic droit (maquette validée le 25/11).
- Charte de design mise à jour (document du projet et `docs/DESIGN.md`) avec les éléments nouveaux et
  modifiés depuis le 09/11.

## D-203 — Première publication gratuite sur GitHub Pages (25/11/2026)

- Audit M11, maquette `tools-5-publish-assistant` validée le 24/11 : un site **sans hébergement** qui
  clique « Publish » ouvre « Publish for free with GitHub Pages » en trois étapes encadrées : (1) compte
  GitHub (relié : coche verte et « Signed in as … » ; sinon le bouton de connexion habituel), (2) nom du
  site (nom du site mis en forme, adresse `https://<compte>.github.io/<nom>/` montrée, nom refusé s'il
  contient autre chose que lettres, chiffres, `-`, `_`, `.`), (3) mise en ligne. « Continue » enregistre
  l'hébergement GitHub Pages (`gh-pages`) et lance la publication : le cœur crée le dépôt public s'il
  n'existe pas et active GitHub Pages (déjà en place, D-161).
- Hébergement payant (SFTP) : ⓘ du titre, réglé dans Paramètres › Publication (« Add hosting », D-202).
- Essai de bout en bout : site sans hébergement (`noprofiles`), trois étapes, « Continue » grisé tant que
  le compte n'est pas relié.
- **Champ « Published » (demande du 25/11)** : là où une collection a un champ `published` (ou `draft`),
  il montre l'état au lieu d'un interrupteur ou d'une coche : « Published » (vert) ou « Archived », dans la
  fiche et dans le tableau ; on le change par « Archive » / « Unarchive » (un seul outil).
- Essai de l'éditeur : marque « remplacé » d'un champ CMS parfois en retard sur le canevas, consignée en
  **P16** (audit) ; l'essai resélectionne le calque si elle tarde.
