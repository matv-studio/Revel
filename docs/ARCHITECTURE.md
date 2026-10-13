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
revel-cli ── revel-project, revel-import, revel-deploy, revel-i18n, revel-model
revel-core ── revel-project, revel-import, revel-deploy, revel-git, revel-protocol, revel-model
revel-project ── revel-render, revel-assets ── revel-markup ── revel-model
revel-import ── revel-markup, revel-render (types), revel-model
revel-protocol, revel-i18n : sans dépendance interne (compilées aussi pour wasm32)
```

- `revel-core` : services de l'application (sites récents, ouvrir, importer, générer, aperçu, hébergements,
  publier), état de l'application (`revel-state.json`, sans secret), mots de passe dans le trousseau.
  Appelé par l'application et, à terme, par la ligne de commande et le serveur MCP (D-13).
- `revel-git` : publication d'un dossier sur une branche Git (dépôt cache nu dans `.revel/git/`, arbre
  construit directement dans la base d'objets, envoi des seuls objets nouveaux, avance rapide uniquement) ;
  client GitHub (device flow, renouvellement des jetons, API Pages) avec `ureq`.
- `revel-protocol` : messages sérialisables entre l'interface et le cœur (résumés, événements de progression,
  échecs `Failure` = clé de traduction + paramètres, D-16). Aucune dépendance système.

## Modèle de site (schéma v1)

- `Site` : pages, gabarits (`Layout`, emboîtables), composants, collections, données globales,
  dossier de données (`data_dir`, fichiers YAML/JSON modifiables par Pages CMS), copies (`passthrough`),
  réglages (typographie, environnements, **filtres personnalisés définis par expression**).
- `Node` : `Element` (attributs, éventuellement conditionnels), `Text`, `Html`, `Out` (expression),
  `If`, `Each`, `Set`, `SetBlock`, `Slot`, `Use` (composant), `Image`.
- Les expressions suivent la syntaxe Jinja et sont évaluées par minijinja. Portée : comme Nunjucks
  (`set` met à jour la variable existante la plus proche).
- Les contenus de collections restent des fichiers Markdown + front matter (compatibles Pages CMS) :
  ils ne sont pas copiés dans `site.revel.json`.

## Projet sur disque

`site.revel.json` à la racine ; les chemins sont relatifs au projet. Un dépôt existant (Eleventy) accueille
ce fichier sans déplacer ses dossiers : pendant la transition, Eleventy et Revel peuvent générer le même site.

## Vérification d'équivalence

`tools/compare-sites.py <référence> <revel>` compare deux sorties : titres, balises meta/link, données
structurées, texte visible, liens, images (nom d'origine, texte alternatif), structure, attributs ; fichiers
texte ligne à ligne.

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
