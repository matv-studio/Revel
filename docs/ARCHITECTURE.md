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
revel-app ──┬─ revel-deploy, revel-i18n, revel-model
revel-cli ──┬─ revel-project, revel-import, revel-deploy, revel-i18n, revel-model
revel-project ── revel-render ── revel-markup ── revel-model
revel-import ──┬─ revel-markup, revel-render (types), revel-model
(autres crates : sans dépendance interne)
```

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
- Interface : **Leptos 0.8** en WebAssembly, construite avec Trunk (D-05). L'interface provisoire en HTML
  statique (`ui/`) sera remplacée à l'étape 1.8.
- Liaison MCP prévue (D-13) : toutes les modifications passeront par des commandes communes à l'interface,
  à la ligne de commande et au futur serveur MCP.
- Permissions Tauri minimales (`capabilities/default.json` : `core:default`).

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
| thiserror / anyhow | 2.0.21 / 1.0.104 | erreurs |

Bibliothèques envisagées pour l'étape 1 : voir `ETAPE-1.md`.
