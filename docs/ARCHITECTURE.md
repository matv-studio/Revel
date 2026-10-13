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

## Dépendances entre modules (étape 0)

```
revel-app ──┬─ revel-deploy
            ├─ revel-i18n
            └─ revel-model
revel-cli ──┬─ revel-model, revel-render, revel-deploy, revel-i18n
revel-render ── revel-model
revel-import ── revel-model
(autres crates : sans dépendance interne)
```

## Coque et interface

- **Tauri 2** (2.12.3 au 13/10/2026) : WebView2 sous Windows, WKWebView sous macOS, WebKitGTK 4.1 sous Linux.
- Interface provisoire en HTML statique (`ui/`), appelant Rust par `invoke`. Le cadre d'interface définitif
  (Leptos ou Dioxus) est une décision ouverte (voir DECISIONS.md, D-05).
- Permissions Tauri minimales (`capabilities/default.json` : `core:default`).

## Versions de référence (vérifiées sur crates.io le 13/10/2026)

| Bibliothèque | Version | Usage |
|---|---|---|
| Rust | 1.99.0 | figé dans `rust-toolchain.toml` |
| tauri / tauri-build / tauri-cli | 2.12.3 / 2.7.2 / 2.12.2 | coque |
| serde / serde_json | 1.0.229 / 1.0.154 | sérialisation |
| fluent-bundle / unic-langid | 0.16.0 / 0.9.6 | traductions |
| clap | 4.6.7 | ligne de commande |
| thiserror / anyhow | 2.0.21 / 1.0.104 | erreurs |

Bibliothèques envisagées pour l'étape 1 : voir `ETAPE-1.md`.
