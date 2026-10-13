# Revel

Application de bureau (Windows, macOS, Linux) de création visuelle de sites web, écrite en Rust.
*Desktop visual website builder written in Rust (English and French UI).*

État : **étape 0** — squelette modulaire, CI multi-plateforme. Rien d'utilisable par un utilisateur final pour l'instant.

## Organisation du dépôt

| Dossier | Rôle |
|---|---|
| `crates/revel-model` | Document de site (schéma versionné, migrations) |
| `crates/revel-render` | Génération HTML/CSS |
| `crates/revel-assets` | Images (AVIF/WebP/JPEG), polices |
| `crates/revel-cms` | Contenus Markdown/YAML compatibles Pages CMS |
| `crates/revel-deploy` | Publication : trait `Provider`, capacités et avertissements par hébergeur |
| `crates/revel-dns` | Noms de domaine (OVH, Cloudflare) |
| `crates/revel-git` | Opérations Git |
| `crates/revel-mcp` | Serveur MCP pour le connecteur Claude |
| `crates/revel-i18n` | Traductions anglais / français (Fluent) |
| `crates/revel-import` | Import de sites existants |
| `apps/revel-app` | Application de bureau (Tauri 2) |
| `apps/revel-cli` | Ligne de commande `revel` (tests, diagnostic) |
| `ui/` | Interface provisoire (HTML statique) |
| `docs/` | Architecture, décisions, plans d'étape |

Détails : [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · Décisions : [docs/DECISIONS.md](docs/DECISIONS.md) · Plan en cours : [docs/ETAPE-1.md](docs/ETAPE-1.md)

## Commandes (développement)

```sh
cargo test --workspace          # tous les tests
cargo run -p revel-cli -- info  # ligne de commande
cargo tauri dev                 # application (depuis apps/revel-app, outil tauri-cli requis)
```

Rust est figé par `rust-toolchain.toml` (installé automatiquement par rustup).

## Revenir en arrière

- Chaque étape terminée reçoit une étiquette Git `step-N` : `git checkout step-0` restaure l'état exact.
- Chaque module est une crate indépendante : on peut revenir sur un seul module
  (`git checkout step-0 -- crates/revel-deploy`) tant que son interface publique (traits) n'a pas changé.
- Les fonctions optionnelles se désactivent à la compilation (ex. `--no-default-features` sur `revel-deploy`).
