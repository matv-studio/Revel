# Journal des décisions

Format : identifiant, date, statut (**prise**, **ouverte**, **à confirmer**), décision et raison.
Une décision prise n'est modifiée que par une nouvelle entrée qui la remplace.

## Décisions prises

**D-01 — 13/10/2026 — prise.** Nom « Revel ». Crate `revel` libre sur crates.io, paquet npm `revel` pris,
domaine `revel-studio.app` libre (vérifié par RDAP le 13/10/2026). Vérification INPI/EUIPO à faire avant diffusion publique.

**D-02 — 13/10/2026 — prise.** Coque Tauri 2, cœur en Rust. Moteur d'affichage du système
(WebView2, WKWebView, WebKitGTK 4.1).

**D-03 — 13/10/2026 — prise.** Espace de travail Cargo, une crate par module (voir ARCHITECTURE.md),
étiquette Git `etape-N` à la fin de chaque étape, CI Windows/macOS/Linux.

**D-04 — 13/10/2026 — prise.** Rust figé à 1.99.0 (`rust-toolchain.toml`), édition 2024.
Rust stable courant d'après `static.rust-lang.org/dist/channel-rust-stable.toml` (publié le 01/10/2026).

**D-06 — 13/10/2026 — prise.** Panneau Claude par connecteur personnalisé claude.ai (MCP distant),
compatible offre gratuite (un connecteur personnalisé). Pas de connexion claude.ai intégrée ni de clé API.
Voir passation, section 4.2.

## Décisions ouvertes

**D-05 — ouverte (à trancher avant l'étape 2).** Cadre d'interface.

| Option | Avantages | Inconvénients |
|---|---|---|
| Leptos (0.8.22) en WebAssembly dans Tauri | Réactivité fine ; intégration Tauri documentée (Trunk) ; proche du modèle web, donc du canevas d'édition | Outil Trunk supplémentaire ; guide Tauri écrit pour Leptos 0.6 ; deux cibles de compilation (natif + wasm) |
| Dioxus (0.7.10) | Outil `dx` tout-en-un (rechargement à chaud du code Rust, empaquetage) ; rendu natif GPU « Blitz » en développement | Remplacerait ou doublerait Tauri (Dioxus a sa propre coque) ; rendu natif incomplet en CSS ; intégration Tauri non documentée par Dioxus |
| HTML/JS sans cadre (actuel) | Aucune dépendance ; rapide à modifier | Interface complexe (éditeur visuel) difficile à maintenir sans cadre ; écart avec « écrite en Rust » |

Sources : https://v2.tauri.app/start/frontend/leptos/ ; https://dioxuslabs.com/blog/release-070/ ; versions crates.io du 13/10/2026.

**D-07 — à confirmer avant la première version publique.** Identifiant d'application `app.revel-studio.desktop`
(`tauri.conf.json`). Il détermine les dossiers de données et les mises à jour : le changer après diffusion casse la continuité.
Basé sur le domaine `revel-studio.app`, libre mais non enregistré.

**D-08 — ouverte.** Licence du code (diffusion publique prévue) : propriétaire, MIT/Apache-2.0, GPL… À décider avant de rendre le dépôt public.

**D-09 — ouverte (étape 6).** Signature de code : certificat Windows (sinon avertissement SmartScreen), compte Apple Developer (sinon blocage Gatekeeper).

**D-10, D-11, D-12 — ouvertes (étape 1).** Stratégie d'import du cobaye, connexion GitHub depuis Revel,
bibliothèque d'envoi Git : options détaillées dans `ETAPE-1.md`.
