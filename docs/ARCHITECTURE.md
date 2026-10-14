# Architecture

## Principes

1. **Un module = une crate.** Les crates communiquent par des interfaces (traits) publiques stables.
   Une crate ne dépend que de `revel-model` et de bibliothèques externes, sauf exception notée ici.
2. **Le modèle est la source de vérité.** Revel n'édite pas du HTML : il édite un document de site
   (`revel-model`) et génère le HTML/CSS (`revel-render`). Pas de Node.js à l'exécution.
3. **Schéma versionné.** Tout changement du format de document passe par une migration (`revel-model/src/migrations.rs`).
   Un document ancien est toujours relu.
4. **Fonctions désactivables** par `features` Cargo (ex. hébergeurs dans `revel-deploy`).
5. **La coque applicative ne contient pas de logique métier** : `apps/revel-app` relie l'interface aux crates.
   La même logique est accessible en ligne de commande (`apps/revel-cli`), ce qui permet de la tester en CI.
6. **Textes d'interface traduits** : aucune chaîne visible en dur ; clés Fluent dans `revel-i18n/locales/{en,fr}`.
   Un test vérifie que les deux langues ont exactement les mêmes clés.

## Dépendances entre modules

```
revel-ui (WebAssembly) ── revel-protocol, revel-i18n
revel-app ── revel-core, revel-protocol, revel-model
revel-cli ── revel-core (sans trousseau), revel-project, revel-import, revel-deploy, revel-i18n, revel-model
revel-core ── revel-project, revel-import, revel-deploy, revel-git, revel-edit, revel-render, revel-assets, revel-protocol, revel-model
revel-edit ── revel-style, revel-model (compilées aussi pour wasm32)
revel-project ── revel-render, revel-assets, revel-style ── revel-markup ── revel-model
revel-render ── revel-edit (types des repères du canevas), revel-markup, revel-model
revel-import ── revel-markup, revel-render (types), revel-style, revel-model
revel-protocol, revel-i18n : sans dépendance interne (compilées aussi pour wasm32)
```

- `revel-core` : services de l'application (sites récents, ouvrir, importer, générer, aperçu, hébergements,
  publier), état de l'application (`revel-state.json`, sans secret), mots de passe dans le trousseau.
  Appelé par l'application et, à terme, par la ligne de commande et le serveur MCP (D-13).
- `revel-edit` : commandes d'édition sérialisables et réversibles (attributs, balise, texte, insertion,
  suppression, déplacement, image, logique, variables de page, suites atomiques ; intentions « dupliquer »,
  « envelopper », « ajouter » développées en commandes concrètes avant l'historique), historique annuler/rétablir
  avec fusion des saisies continues. Sessions d'édition dans `revel-core` (`edit_open`, `edit_apply`, `edit_undo`,
  `edit_redo`) : enregistrement de `site.revel.json` à chaque modification ; ligne de commande `revel apply`.
  Commandes de style : déclaration (`SetDecl`, valeur changée sur place, ajoutée ou retirée), sélecteur,
  insertion et retrait de règles ; intention « nouvelle règle » placée dans la cascade (`AddRule`). Les valeurs
  sont relues par `revel-style` avant d'être acceptées (une seule déclaration, pas de `}` ni de `;` hors chaîne).
- `revel-style` (D-18) : lecture d'un CSS existant en éléments du modèle (règles, règles dans `@media`,
  commentaires, blocs bruts pour `@font-face`, `@keyframes`, `@supports`, CSS imbriqué), sans réécriture des
  valeurs ; génération du CSS publié (une ligne par règle, règles consécutives d'une même requête regroupées) ;
  forme canonique pour les comparaisons (`revel css-canon`) ; points de rupture déduits des `(max-width)` et
  `(min-width)` ; position d'une nouvelle règle dans la cascade. Lecteur propre plutôt que lightningcss : ce
  dernier réécrit les valeurs (couleurs, raccourcis), ce qui changerait le CSS de l'utilisateur.
- Identifiants des nœuds (schéma 2) : chaque nœud structurel (élément, image, condition, boucle, composant
  utilisé, bloc) a un identifiant unique dans le site, déterministe à l'import (`revel_model::ids`).
- `revel-git` : publication d'un dossier sur une branche Git (dépôt cache nu dans `.revel/git/`, arbre
  construit directement dans la base d'objets, envoi des seuls objets nouveaux, avance rapide uniquement) ;
  client GitHub (device flow, renouvellement des jetons, API Pages) avec `ureq`.
- `revel-protocol` : messages sérialisables entre l'interface et le cœur (résumés, événements de progression,
  échecs `Failure` = clé de traduction + paramètres, D-16). Aucune dépendance système.

## Modèle de site (schéma 3)

- `Site` : pages, gabarits (`Layout`, emboîtables), composants, collections, données globales,
  dossier de données (`data_dir`, fichiers YAML/JSON modifiables par Pages CMS), copies (`passthrough`),
  réglages (typographie, environnements, **filtres personnalisés définis par expression**).
- `Node` : `Element` (attributs, éventuellement conditionnels), `Text`, `Html`, `Out` (expression),
  `If`, `Each`, `Set`, `SetBlock`, `Slot`, `Use` (composant), `Image`.
- Les expressions suivent la syntaxe Jinja et sont évaluées par minijinja. Portée : comme Nunjucks
  (`set` met à jour la variable existante la plus proche).
- Les contenus de collections restent des fichiers Markdown + front matter (compatibles Pages CMS) :
  ils ne sont pas copiés dans `site.revel.json`.
- `Stylesheet` (schéma 3) : adresse publiée (`url`), fichier d'origine importé (`source`, non copié à la
  génération) et liste ordonnée d'éléments (`Rule` avec identifiant, sélecteur, requête média, déclarations ;
  `Comment` ; `Raw`). `breakpoints` : points de rupture du site. Migration 2 → 3 : les anciennes classes
  (`styles`) deviennent des règles.

## Projet sur disque

`site.revel.json` à la racine ; les chemins sont relatifs au projet. Un dépôt existant (Eleventy) accueille
ce fichier sans déplacer ses dossiers : pendant la transition, Eleventy et Revel peuvent générer le même site.

## Vérification d'équivalence

`tools/compare-sites.py <référence> <revel>` compare deux sorties : titres, balises meta/link, données
structurées, texte visible, liens, images (nom d'origine, texte alternatif), structure, attributs ; fichiers
texte ligne à ligne ; autres fichiers (médias, CSS, JS, polices) : même liste et contenu identique, hors variantes
d'images (`img/`) ; feuilles de style comparées sous forme canonique (`REVEL_BIN` = binaire `revel`). Le job CI
*Équivalence avec Eleventy* l'applique au cobaye (`matv-studio/alunites-revel-lab`, version fixée par
`COBAYE_REF`) en production et en recette.

`tools/compare-styles.mjs <a> <b>` vérifie que deux feuilles de style donnent les mêmes styles : le même site
servi avec le CSS d'origine et avec le CSS généré, ouvert dans Chromium (Playwright) à 1280 px, à chaque point
de rupture et à 375 px, avec et sans préférence « mouvement réduit » ; après chargement et décodage des images,
arrêt des vidéos et fin des animations, les styles calculés de chaque élément et de ses pseudo-éléments sont
comparés (ensemble des propriétés : l'ordre des propriétés personnalisées varie d'une ouverture à l'autre).
Avec `--shots`, les captures pleine page sont aussi comparées : le dessin des images mises à l'échelle et du texte
tourné varie d'une capture à l'autre d'un même site (jusqu'à 30 sur 255 par canal mesuré), d'où une tolérance de
48 par canal ; une paire en écart est reprise une fois avant d'être signalée (`--save` garde les captures, jointes à
l'exécution CI en cas d'échec).

## Coque et interface

- **Tauri 2** (2.12.3 au 13/10/2026) : WebView2 sous Windows, WKWebView sous macOS, WebKitGTK 4.1 sous Linux.
- Interface : **Leptos 0.8** en WebAssembly (`apps/revel-ui`, espace de travail séparé), construite avec Trunk
  (D-05) ; système de design dans `docs/DESIGN.md` (D-15). Les commandes Tauri (`apps/revel-app/src/lib.rs`) ne
  font que relayer `revel-core` ; la progression passe par des canaux Tauri (`Channel`).
- Les messages d'un canal peuvent arriver après la réponse de la commande : l'interface fixe l'état final au
  premier signal de fin et ignore la progression qui suit.
- Liaison MCP prévue (D-13) : toutes les modifications passeront par des commandes communes à l'interface,
  à la ligne de commande et au futur serveur MCP.
- Permissions Tauri minimales (`capabilities/default.json` : `core:default`, `dialog:allow-open`).
- Site servi dans un sous-dossier (GitHub Pages sans domaine personnalisé) : `revel_render::rebase` préfixe
  les adresses depuis la racine (attributs, `srcset`, `meta refresh`, `url()` CSS) dans une copie liée du site
  généré (`revel_project::rebase_dir`) ; le site généré lui-même ne change pas.
- Générations reproductibles : le paramètre `?v=` des CSS/JS est l'empreinte de leur contenu ; deux générations
  sans changement produisent des fichiers identiques, et la publication suivante n'envoie rien.

## Canevas de l'éditeur (étape 2.2)

- Page rendue à la demande depuis le document en cours d'édition (`revel_render::render_editor`), même code que
  la génération, avec des repères : `data-rv="<identifiant>"` sur chaque élément ; commentaires `<!--rv:N-->` …
  `<!--/rv:N-->` autour des textes, des expressions affichées, des composants utilisés et des images remplacées.
  La liste des repères (`revel_edit::canvas::Slot`) donne l'adresse de chaque texte (`SetText`) et l'origine de
  chaque valeur affichée (D-19) : données globales (chemin dans le dossier de données), variable de page ou de
  gabarit, en-tête d'un contenu de collection. L'origine suit les `set`, les boucles (indice ou clé) et les
  éléments retrouvés par un filtre (élément de collection reconnu, objet présent une seule fois dans les
  données) ; une valeur transformée par des filtres est reliée à sa source avec `transformed`.
- Pas de repère dans un attribut, dans `<title>`, `<textarea>`, `<script>`, `<style>`, ni dans un bloc capturé
  (`{% set %}…{% endset %}`). La typographie française ignore les repères. Repères retirés, chaque page est
  identique octet pour octet à la page générée (`revel canvas-check`, essais de `revel-render`, CI).
- Scripts du site neutralisés par défaut (`type="text/plain"`, sauf données JSON) ; option pour les exécuter.
- Servi par le protocole `revel-canvas` de l'application (`revel-canvas://localhost/…`, sous Windows
  `http://revel-canvas.localhost/…`), sans port réseau : page, feuille de style générée depuis le document
  (modifications visibles aussitôt), variante d'image produite ou image d'origine, copies du projet ; chemins
  hors du site refusés. `revel canvas-serve` sert les mêmes réponses en HTTP local (diagnostic, essais).
- Script du canevas (`crates/revel-core/src/canvas.js`) : lit la liste des repères puis la retire de la page avec
  son propre `<script>`, remplace les commentaires par des plages (`Range`) vivantes et refusionne les textes, si
  bien que la page a la même structure que le site (styles calculés et captures identiques, vérifié en CI) ;
  calque de survol et de sélection dans un shadow DOM fermé ; les clics sélectionnent sans déclencher le site ;
  échanges avec l'interface par `postMessage` (`ready`, `hover`, `select`, `key` ; demandes `select`, `hover`,
  `stylesheet`, `scroll`). Un composant utilisé plusieurs fois a les mêmes identifiants dans chaque occurrence :
  l'élément cliqué est celui qui est sélectionné, une sélection demandée par identifiant prend la première.
- Le cadre du canevas est d'une autre origine que l'interface (`frame-src` dans la CSP de l'application).
- Interface de l'éditeur (`apps/revel-ui/src/editor.rs`, étape 2.3) : copie locale du document (`revel-model`,
  `revel-edit` compilés en WebAssembly) ; chaque modification est une commande envoyée au cœur, la commande
  appliquée renvoyée (`EditState::applied`) est rejouée sur la copie. Une commande de style ne recharge que la
  feuille de style du canevas ; les autres rechargent la page au même défilement, sélection conservée.
- Calques (`apps/revel-ui/src/layers.rs`) : arbre calculé depuis la copie locale (gabarit extérieur, `Slot`
  remplacé par le gabarit intérieur puis la page, composants dépliés sous leur utilisation) ; chaque ligne
  connaît sa liste (`Parent`) et sa position, d'où les commandes `Move`, `Duplicate`, `Wrap`, `Remove`. Un
  déplacement reste dans le même propriétaire (page, gabarit ou composant).
- Textes (étape 2.7) : le script du canevas rend un repère texte modifiable (`contenteditable`) au
  double-clic et renvoie le texte ; l'interface en fait `SetText` ou `SetBound`. `SetBound` est appliquée
  par le cœur (`History::apply_with` et son exécutant) : `revel_project::bound` retrouve le fichier (dossier
  de données : `pages.accueil.hero.title` = `pages/accueil.yml`, clé `hero.title` ; contenu : en-tête du
  fichier Markdown) et remplace la seule valeur visée à l'aide des positions des événements YAML
  (`granit-parser`, utilisé aussi par `serde-saphyr`). L'inverse remet l'ancienne valeur, d'où une annulation
  à l'octet près. `edit_stamp` donne une empreinte des fichiers lus par le canevas, comparée par
  l'interface toutes les 2 s.
- Essais de l'interface dans un navigateur : `revel canvas-serve --bridge` expose aussi les commandes d'édition
  (`POST /__revel/invoke/<commande>`, adresse locale seulement) ; l'API Tauri simulée
  (`apps/revel-ui/dev/mock-tauri.js`, paramètre `bridge=`) les y transmet. `tools/editor-e2e.mjs` pilote
  l'interface ainsi reliée à un vrai cœur (CI). Le crochet `window.__revelTest` n'existe qu'avec la simulation.

## Versions de référence (vérifiées sur crates.io le 13/10/2026)

| Bibliothèque | Version | Usage |
|---|---|---|
| Rust | 1.99.0 | figé dans `rust-toolchain.toml` |
| tauri / tauri-build / tauri-cli | 2.12.3 / 2.7.2 / 2.12.2 | coque |
| serde / serde_json | 1.0.229 / 1.0.154 | sérialisation |
| fluent-bundle / unic-langid | 0.16.0 / 0.9.6 | traductions |
| clap | 4.6.7 | ligne de commande |
| minijinja | 3.0.0 | expressions (syntaxe Jinja) |
| pulldown-cmark | 0.13.4 | Markdown |
| serde-saphyr | 1.3.0 | YAML (front matter, données) |
| htmlize | 1.1.0 | entités HTML |
| regex | 1.13.1 | filtres et import |
| chrono | 0.4.45 | dates |
| image / fast_image_resize | 0.25.10 / 6.1.0 | décodage, redimensionnement |
| ravif / webp / jpeg-encoder | 0.13.0 / 0.3.1 / 0.7.1 | encodage AVIF, WebP, JPEG |
| russh / russh-sftp | 0.64.1 / 3.0.1 | SFTP (cryptographie `ring`) |
| sha1 | 0.11.0 | manifeste de publication |
| thiserror / anyhow | 2.0.21 / 1.0.104 | erreurs |

Bibliothèques envisagées pour l'étape 1 : voir `ETAPE-1.md`.
