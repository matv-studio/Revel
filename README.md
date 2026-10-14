# Revel

Application de bureau (Windows, macOS, Linux) de création visuelle de sites web, écrite en Rust.
*Desktop visual website builder written in Rust (English and French UI).*

État : **étape 1** terminée (version 0.1.0) — ouvrir un site existant (import Eleventy), le générer (images
AVIF/WebP), le prévisualiser et le publier par SFTP ou sur GitHub Pages, depuis l'application (français,
anglais) ; sortie vérifiée équivalente à celle d'Eleventy à chaque envoi (CI).
L'éditeur visuel arrive à l'étape 2.

## Organisation du dépôt

| Dossier | Rôle |
|---|---|
| `crates/revel-model` | Document de site (schéma versionné, migrations) |
| `crates/revel-render` | Génération HTML/CSS |
| `crates/revel-assets` | Images (AVIF/WebP/JPEG), polices |
| `crates/revel-cms` | Contenus Markdown/YAML compatibles Pages CMS |
| `crates/revel-deploy` | Publication : trait `Provider`, capacités et avertissements par hébergeur |
| `crates/revel-dns` | Noms de domaine (OVH, Cloudflare) |
| `crates/revel-git` | Git et GitHub : publication sur une branche, connexion (device flow), GitHub Pages |
| `crates/revel-project` | Projet sur disque : document de site, contenus, génération |
| `crates/revel-markup` | Lecture des gabarits (Nunjucks/Jinja) vers l'arbre de Revel |
| `crates/revel-core` | Services de l'application (communs à l'interface, à la ligne de commande et au futur MCP) |
| `crates/revel-protocol` | Messages entre l'interface et le cœur |
| `crates/revel-mcp` | Serveur MCP pour le connecteur Claude |
| `crates/revel-i18n` | Traductions anglais / français (Fluent) |
| `crates/revel-import` | Import de sites existants |
| `apps/revel-app` | Application de bureau (Tauri 2) |
| `apps/revel-ui` | Interface (Leptos, WebAssembly) ; système de design : [docs/DESIGN.md](docs/DESIGN.md) |
| `apps/revel-cli` | Ligne de commande `revel` (tests, diagnostic) |
| `docs/` | Architecture, décisions, plans d'étape |

Détails : [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · Décisions : [docs/DECISIONS.md](docs/DECISIONS.md) · Plan en cours : [docs/ETAPE-1.md](docs/ETAPE-1.md)

## Commandes (développement)

```sh
cargo test --workspace          # tous les tests
cargo run -p revel-cli -- info  # ligne de commande
cargo tauri dev                 # application (depuis apps/revel-app ; tauri-cli et trunk requis)
```

CI : Linux, Windows et macOS à chaque envoi (dépôt public : minutes gratuites). Installateurs : workflow
*Installateurs*, lancé à la main avec le choix des systèmes. Étiquettes, branches, exécutions : workflows
*Étiquette* et *Maintenance* (la session de Claude ne peut pas les modifier directement).

## Licence

Aucune licence pour l'instant (décision D-08 ouverte) : le code est visible, mais tous les droits sont
réservés ; il ne peut être ni copié, ni redistribué, ni modifié sans autorisation.

Rust est figé par `rust-toolchain.toml` (installé automatiquement par rustup).

## Revenir en arrière

- Chaque étape terminée reçoit une étiquette Git `step-N` : `git checkout step-0` restaure l'état exact.
- Chaque module est une crate indépendante : on peut revenir sur un seul module
  (`git checkout step-0 -- crates/revel-deploy`) tant que son interface publique (traits) n'a pas changé.
- Les fonctions optionnelles se désactivent à la compilation (ex. `--no-default-features` sur `revel-deploy`).
